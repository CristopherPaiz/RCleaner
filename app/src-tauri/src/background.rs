//! Análisis periódico en segundo plano.
//!
//! Cada `every_hours` analiza con el modo por defecto (sólo lee, no borra) y,
//! si lo que se puede limpiar sin riesgo supera el umbral, avisa con una
//! notificación de Windows. El resultado queda en el icono de la bandeja.

use crate::state::{AppState, BackgroundStatus, lock};
use crp_core::settings::unix_now;
use crp_core::{NoProgress, Risk, tf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// Primer análisis tras arrancar (deja que Windows termine de iniciar).
const FIRST_DELAY: Duration = Duration::from_secs(120);
const TICK: Duration = Duration::from_secs(60);

/// Evita dos análisis a la vez (el periódico y el del menú).
static RUNNING: Mutex<()> = Mutex::new(());

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut next = Instant::now() + FIRST_DELAY;
        loop {
            std::thread::sleep(TICK);
            let bg = lock(&app.state::<AppState>().settings).background.clone();
            if !bg.enabled || Instant::now() < next {
                continue;
            }
            check(&app, false);
            next = Instant::now()
                + Duration::from_secs(u64::from(bg.every_hours.clamp(1, 168)) * 3600);
        }
    });
}

/// Análisis pedido desde la bandeja: siempre avisa del resultado.
pub fn check_now(app: AppHandle) {
    std::thread::spawn(move || check(&app, true));
}

fn check(app: &AppHandle, always_notify: bool) {
    let Ok(_guard) = RUNNING.try_lock() else {
        return;
    };
    let state = app.state::<AppState>();
    let engine = lock(&state.engine).clone();
    let ctx = crp_modules::system_context().with_settings(&lock(&state.settings));
    let selectors = state.default_selectors();
    let report = engine.scan(&selectors, &ctx, &NoProgress);

    let safe: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.selected && f.risk == Risk::Safe && (ctx.is_admin || !f.requires_admin))
        .collect();
    let status = BackgroundStatus {
        timestamp: unix_now(),
        bytes: safe.iter().map(|f| f.bytes).sum(),
        findings: safe.len(),
    };
    *lock(&state.background) = Some(status);
    drop(report);

    let size = format_bytes(status.bytes);
    crate::tray::set_tooltip(app, &tf("app.tray.tooltip", &[("size", &size)]));

    let threshold = lock(&state.settings).background.threshold_mb * 1024 * 1024;
    if always_notify || status.bytes >= threshold {
        let body = if status.bytes > 0 {
            tf("app.notify.body", &[("size", &size)])
        } else {
            crp_core::t("app.notify.clean")
        };
        let _ = app
            .notification()
            .builder()
            .title(tf("app.notify.title", &[("size", &size)]))
            .body(body)
            .show();
    }
    if app.get_webview_window("main").is_none() {
        crp_platform::process::trim_memory();
    }
}

/// `1.4 GB`, `820 MB`… (para textos fuera de la UI).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 || v >= 100.0 {
        format!("{v:.0} {}", UNITS[u])
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}
