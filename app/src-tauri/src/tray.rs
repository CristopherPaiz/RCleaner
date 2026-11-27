//! Ventana principal y bandeja del sistema.
//!
//! Para que RCleaner ocupe lo mínimo cuando está en la bandeja, la ventana no
//! se oculta: se **destruye** (WebView2 y sus procesos se cierran) y se
//! liberan los análisis grandes. Al volver se crea de nuevo desde la
//! configuración de `tauri.conf.json`; el estado importante vive en Rust.

use crate::state::{AppState, lock};
use crp_core::t;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindowBuilder, Window, WindowEvent, Wry};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

const TRAY_ID: &str = "rcleaner";
const MAIN: &str = "main";

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", t("app.tray.open"), true, None::<&str>)?,
            &MenuItem::with_id(app, "check", t("app.tray.check"), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", t("app.tray.quit"), true, None::<&str>)?,
        ],
    )
}

/// Icono de la bandeja con su menú. Siempre visible mientras la app corre.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("RCleaner")
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show(app),
            "check" => crate::background::check_now(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Textos del menú en el idioma actual.
pub fn refresh_menu(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn set_tooltip(app: &AppHandle, text: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(text));
    }
}

/// Muestra la ventana principal (creándola si estaba en la bandeja).
pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let Some(cfg) = app.config().app.windows.iter().find(|w| w.label == MAIN) else {
        return;
    };
    match WebviewWindowBuilder::from_config(app, cfg).and_then(|b| b.build()) {
        Ok(w) => {
            let _ = w.set_focus();
        }
        Err(e) => eprintln!("no se pudo abrir la ventana: {e}"),
    }
}

/// Cierra la ventana y deja la app en la bandeja con la mínima memoria. Si
/// hay una tarea en curso, sólo la oculta (para no perder su resultado).
pub fn hide(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(w) = app.get_webview_window(MAIN) else {
        return;
    };
    if state.is_busy() {
        let _ = w.hide();
        return;
    }
    state.to_tray.store(true, Ordering::SeqCst);
    // Guardar tamaño y posición antes de destruirla (se restauran al volver).
    let _ = app.save_window_state(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED);
    let _ = w.destroy();
}

/// Tras cerrar la ventana: soltar análisis grandes y devolver memoria.
fn release(app: &AppHandle) {
    app.state::<AppState>().drop_heavy();
    std::thread::spawn(|| {
        // WebView2 tarda un poco en cerrar sus procesos.
        for _ in 0..3 {
            std::thread::sleep(Duration::from_millis(800));
            crp_platform::process::trim_memory();
        }
    });
}

/// Minimizar con "bandeja" activado → a la bandeja.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != MAIN {
        return;
    }
    let app = window.app_handle().clone();
    match event {
        WindowEvent::Resized(_) => {
            let tray = lock(&app.state::<AppState>().settings).tray;
            if tray && window.is_minimized().unwrap_or(false) {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || hide(&handle));
            }
        }
        WindowEvent::Destroyed => release(&app),
        _ => {}
    }
}

/// Al cerrarse la última ventana: seguir en la bandeja si se pidió (o si el
/// análisis en segundo plano está activo); si no, salir.
pub fn keep_running(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let to_tray = state.to_tray.swap(false, Ordering::SeqCst);
    to_tray || lock(&state.settings).background.enabled
}
