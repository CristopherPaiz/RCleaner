// Sin consola en release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Todo el código de sistema (`unsafe`/FFI) vive sólo en crp-platform.
#![forbid(unsafe_code)]

mod background;
mod commands;
mod icons;
mod space;
mod state;
mod tray;

use state::lock;
use std::time::Duration;
use tauri_plugin_autostart::MacosLauncher;

/// Argumento con el que Windows abre RCleaner al iniciar sesión: sólo bandeja.
pub const BACKGROUND_ARG: &str = "--background";
/// `--wait-pid <pid>`: esperar a que cierre la instancia que nos relanzó.
pub const WAIT_ARG: &str = "--wait-pid";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pid) = args
        .iter()
        .position(|a| a == WAIT_ARG)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
    {
        crp_platform::process::wait_for_exit(pid, Duration::from_secs(10));
    }
    let background = args.iter().any(|a| a == BACKGROUND_ARG);
    let state = state::AppState::new();

    // "Abrir siempre como administrador": pedirlo antes de crear nada. Al
    // iniciar con Windows no (sería un UAC en cada arranque); si el usuario
    // cancela el UAC, se sigue sin permisos.
    let always_admin = lock(&state.settings).always_admin;
    // En desarrollo no: el proceso elevado se separa de `cargo tauri dev`.
    if always_admin
        && !cfg!(debug_assertions)
        && !background
        && !crp_platform::is_elevated()
        && crp_platform::relaunch_elevated(&[]).is_ok()
    {
        return;
    }

    let builder = tauri::Builder::default();
    // Primero: una segunda instancia sólo trae al frente la primera. En
    // desarrollo no, para poder probar con la versión instalada abierta.
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| {
        tray::show(app)
    }));
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![BACKGROUND_ARG]),
        ))
        .manage(state)
        .setup(move |app| {
            tray::create(app.handle())?;
            background::start(app.handle().clone());
            if !background {
                tray::show(app.handle());
            }
            Ok(())
        })
        .on_window_event(tray::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::system_info,
            commands::drives,
            commands::history,
            commands::deletions,
            commands::deletions_count,
            commands::exclusion_sizes,
            commands::trim_deletions,
            commands::background_status,
            commands::recycle_bin,
            commands::list_modes,
            commands::save_mode,
            commands::delete_mode,
            commands::list_modules,
            commands::get_settings,
            commands::save_settings,
            commands::pick_folder,
            commands::scan,
            commands::clean,
            commands::cancel,
            commands::reveal,
            commands::finding_path,
            commands::open_link,
            commands::to_tray,
            commands::restart_as_admin,
            icons::icons,
            space::disk_scan,
            space::disk_summary,
            space::disk_view,
            space::disk_treemap,
            space::disk_children,
            space::disk_crumbs,
            space::disk_largest,
            space::disk_path,
            space::disk_entry,
            space::disk_reveal,
            space::disk_delete,
            space::dupes_scan,
            space::dupes_report,
            space::dupes_path,
            space::dupes_reveal,
            space::dupes_delete,
        ])
        .build(tauri::generate_context!())
        .expect("error al iniciar RCleaner")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                if tray::keep_running(app) {
                    api.prevent_exit();
                }
            }
        });
}
