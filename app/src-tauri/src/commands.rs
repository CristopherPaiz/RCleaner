//! Comandos expuestos al frontend. Todo el trabajo pesado corre en hilos de
//! bloqueo para no congelar la ventana; el progreso se emite como evento.

use crate::state::{AppState, BackgroundStatus, lock};
use crp_core::context::default_dev_roots;
use crp_core::settings::{
    DELETION_LOG_MAX, append_deletions, append_history, count_deletions, load_deletions,
    load_history, unix_now,
};
use crp_core::{
    Background, CancelToken, CleanOptions, CleanReport, DeletionRecord, Env, HistoryEntry, Mode,
    ModuleMeta, ProgressEvent, ScanReport, Settings, modes, t, tf,
};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

const PROGRESS_EVENT: &str = "crp://progress";
/// Ruta que se está eliminando ahora mismo (`{ "path": … }`).
pub const NOW_EVENT: &str = "crp://now";

/// Mientras `running`, emite cada 100 ms la ruta anotada en `token`.
pub fn now_ticker(app: AppHandle, token: CancelToken) -> Arc<AtomicBool> {
    #[derive(Serialize, Clone)]
    struct Now {
        path: Option<PathBuf>,
    }
    let running = Arc::new(AtomicBool::new(true));
    let flag = running.clone();
    std::thread::spawn(move || {
        let mut last = None;
        while flag.load(Ordering::Relaxed) {
            let path = token.current();
            if path != last {
                let _ = app.emit(NOW_EVENT, Now { path: path.clone() });
                last = path;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = app.emit(NOW_EVENT, Now { path: None });
    });
    running
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ── Información ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SystemInfo {
    is_admin: bool,
    home: PathBuf,
    /// Carpetas de proyectos que se usarán (ajustes o detección).
    dev_roots: Vec<PathBuf>,
    /// Carpetas que se detectan automáticamente.
    auto_dev_roots: Vec<PathBuf>,
    config_dir: PathBuf,
    version: &'static str,
    /// Idioma efectivo de los textos del motor.
    language: String,
}

#[tauri::command]
pub fn system_info(state: State<'_, AppState>) -> SystemInfo {
    let ctx = crp_modules::system_context().with_settings(&lock(&state.settings));
    SystemInfo {
        is_admin: ctx.is_admin,
        auto_dev_roots: default_dev_roots(&Env::from_system()),
        home: ctx.env.home,
        dev_roots: ctx.dev_roots,
        config_dir: state.paths.config_dir.clone(),
        version: env!("CARGO_PKG_VERSION"),
        language: crp_core::i18n::language(),
    }
}

#[derive(Serialize)]
pub struct Drive {
    root: String,
    label: String,
    total_bytes: u64,
    free_bytes: u64,
}

#[tauri::command]
pub fn drives() -> Vec<Drive> {
    crp_platform::drives::fixed_drives()
        .into_iter()
        .map(|d| Drive {
            root: d.root,
            label: d.label,
            total_bytes: d.total_bytes,
            free_bytes: d.free_bytes,
        })
        .collect()
}

#[tauri::command]
pub fn history(state: State<'_, AppState>) -> Vec<HistoryEntry> {
    load_history(&state.paths.history_file())
}

/// Registro de lo eliminado, lo más reciente primero.
#[tauri::command]
pub fn deletions(state: State<'_, AppState>, limit: Option<usize>) -> Vec<DeletionRecord> {
    load_deletions(
        &state.paths.deletions_file(),
        limit.unwrap_or(500).min(10_000),
    )
}

/// Último análisis en segundo plano (si lo hubo).
#[tauri::command]
pub fn background_status(state: State<'_, AppState>) -> Option<BackgroundStatus> {
    *lock(&state.background)
}

#[derive(Serialize)]
pub struct RecycleBin {
    bytes: u64,
    items: u64,
}

/// Tamaño de la papelera de reciclaje (todas las unidades).
#[tauri::command]
pub fn recycle_bin() -> Option<RecycleBin> {
    crp_platform::recycle_bin::query().ok().map(|r| RecycleBin {
        bytes: r.bytes,
        items: r.items,
    })
}

/// Guarda lo eliminado en el registro (no interrumpe si falla).
pub fn log_deletions(state: &AppState, records: &[DeletionRecord]) {
    let keep = lock(&state.settings).deletion_log;
    let _ = append_deletions(&state.paths.deletions_file(), records, keep);
}

/// Cuántos borrados hay guardados en el registro.
#[tauri::command]
pub fn deletions_count(state: State<'_, AppState>) -> usize {
    count_deletions(&state.paths.deletions_file())
}

/// Deja sólo los `keep` registros más recientes (`0` = vaciar el registro).
#[tauri::command]
pub fn trim_deletions(state: State<'_, AppState>, keep: u32) -> CmdResult<usize> {
    crp_core::settings::trim_deletions(&state.paths.deletions_file(), keep).map_err(err)
}

// ── Modos y módulos ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ModeList {
    modes: Vec<Mode>,
    /// Modos de usuario que no se pudieron leer.
    errors: Vec<String>,
}

#[tauri::command]
pub fn list_modes(state: State<'_, AppState>) -> ModeList {
    let (modes, errors) = modes::all(&state.paths.modes_dir());
    ModeList { modes, errors }
}

#[tauri::command]
pub fn save_mode(state: State<'_, AppState>, mode: Mode) -> CmdResult<()> {
    modes::save_user(&state.paths.modes_dir(), &mode)
        .map(|_| ())
        .map_err(err)
}

#[tauri::command]
pub fn delete_mode(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    modes::delete_user(&state.paths.modes_dir(), &id).map_err(err)
}

#[tauri::command]
pub fn list_modules(state: State<'_, AppState>) -> Vec<ModuleMeta> {
    lock(&state.engine).metas().cloned().collect()
}

// ── Ajustes ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    lock(&state.settings).clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> CmdResult<()> {
    let clean = |paths: Vec<PathBuf>| -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        paths
            .into_iter()
            .filter(|p| p.is_absolute())
            .filter(|p| seen.insert(p.to_string_lossy().to_lowercase()))
            .collect()
    };
    let settings = Settings {
        dev_roots: clean(settings.dev_roots),
        exclusions: clean(settings.exclusions),
        default_mode: settings.default_mode,
        language: settings.language.filter(|l| !l.is_empty()),
        theme: settings.theme.filter(|t| t == "light" || t == "dark"),
        always_admin: settings.always_admin,
        tray: settings.tray,
        autostart: settings.autostart,
        background: Background {
            enabled: settings.background.enabled,
            every_hours: settings.background.every_hours.clamp(1, 168),
            threshold_mb: settings.background.threshold_mb.clamp(50, 1024 * 1024),
        },
        allow_system: settings.allow_system,
        onboarded: settings.onboarded,
        deletion_log: settings.deletion_log.min(DELETION_LOG_MAX),
        privacy: settings.privacy,
        check_updates: settings.check_updates,
    };
    let previous = lock(&state.settings).clone();
    if previous.autostart != settings.autostart {
        let launcher = app.autolaunch();
        let r = if settings.autostart {
            launcher.enable()
        } else {
            launcher.disable()
        };
        r.map_err(|e| tf("app.autostart_failed", &[("error", &e)]))?;
    }
    settings.save(&state.paths.settings_file()).map_err(err)?;
    let language_changed = previous.language != settings.language;
    if language_changed {
        crate::state::apply_language(&settings);
        // Los nombres de módulos y hallazgos se generan en el idioma activo.
        *lock(&state.engine) = crp_modules::engine();
        lock(&state.last_scan).clear();
    }
    *lock(&state.settings) = settings;
    if language_changed {
        crate::tray::refresh_menu(&app);
    }
    Ok(())
}

/// Selector nativo de carpeta. `None` si el usuario cancela.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> CmdResult<Option<PathBuf>> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
    })
    .await
    .map_err(err)
}

// ── Escaneo y limpieza ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn scan(
    app: AppHandle,
    state: State<'_, AppState>,
    selectors: Vec<String>,
) -> CmdResult<ScanReport> {
    let _busy = state.busy();
    let engine = lock(&state.engine).clone();
    let ctx = state.fresh_context();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let sink = move |e: ProgressEvent| {
            let _ = app.emit(PROGRESS_EVENT, e);
        };
        engine.scan(&selectors, &ctx, &sink)
    })
    .await
    .map_err(err)?;

    *lock(&state.last_scan) = report.findings.clone();
    Ok(report)
}

#[tauri::command]
pub async fn clean(
    app: AppHandle,
    state: State<'_, AppState>,
    finding_ids: Vec<String>,
    dry_run: bool,
    mode_name: Option<String>,
) -> CmdResult<CleanReport> {
    let wanted: HashSet<String> = finding_ids.into_iter().collect();
    let findings: Vec<_> = lock(&state.last_scan)
        .iter()
        .filter(|f| wanted.contains(&f.id))
        .cloned()
        .collect();
    if findings.len() != wanted.len() {
        return Err(t("app.stale_scan"));
    }

    let _busy = state.busy();
    let engine = lock(&state.engine).clone();
    let ctx = state.fresh_context();
    let ticker = now_ticker(app.clone(), ctx.cancel.clone());
    let cleaned = findings.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let sink = move |e: ProgressEvent| {
            let _ = app.emit(PROGRESS_EVENT, e);
        };
        engine.clean(&findings, CleanOptions { dry_run }, &ctx, &sink)
    })
    .await;
    ticker.store(false, Ordering::Relaxed);
    let report = report.map_err(err)?;

    if !dry_run {
        let now = unix_now();
        let records: Vec<DeletionRecord> = report
            .outcomes
            .iter()
            .filter(|o| o.freed_bytes > 0 || o.removed_items > 0)
            .filter_map(|o| {
                let f = cleaned.iter().find(|f| f.id == o.finding_id)?;
                Some(DeletionRecord {
                    timestamp: now,
                    source: "clean".into(),
                    path: f.path.clone().unwrap_or_else(|| PathBuf::from(&f.label)),
                    bytes: o.freed_bytes,
                    permanent: true,
                })
            })
            .collect();
        log_deletions(&state, &records);
        let entry = HistoryEntry::now(
            mode_name,
            report.freed_bytes(),
            report.outcomes.len(),
            report.skipped_items(),
        );
        // El historial es informativo: un fallo al guardarlo no invalida la limpieza.
        let _ = append_history(&state.paths.history_file(), entry);
        // Lo limpiado ya no existe: forzar un nuevo análisis antes de otra limpieza.
        lock(&state.last_scan).clear();
    }
    Ok(report)
}

/// Cancela una tarea en curso: `clean` (análisis/limpieza), `disk` o `dupes`.
#[tauri::command]
pub fn cancel(state: State<'_, AppState>, task: Option<String>) {
    let slot = match task.as_deref() {
        Some("disk") => &state.disk_cancel,
        Some("dupes") => &state.dupes_cancel,
        Some("delete") => &state.delete_cancel,
        _ => &state.cancel,
    };
    lock(slot).cancel();
}

/// Abre el Explorador con la ruta seleccionada. Sólo rutas conocidas.
#[tauri::command]
pub fn reveal(state: State<'_, AppState>, path: PathBuf) -> CmdResult<()> {
    let known = lock(&state.last_scan)
        .iter()
        .any(|f| f.path.as_ref() == Some(&path))
        || lock(&state.settings).dev_roots.contains(&path)
        || lock(&state.settings).exclusions.contains(&path)
        || state.paths.config_dir == path;
    if !known {
        return Err(t("app.unknown_path"));
    }
    crate::space::open_in_explorer(&path)
}

#[derive(Serialize)]
pub struct ExclusionSize {
    path: PathBuf,
    /// `false` si la carpeta ya no existe.
    exists: bool,
    bytes: u64,
    items: u64,
}

/// Tamaño de cada exclusión (sólo lee). Se miden a la vez, una por hilo.
#[tauri::command]
pub async fn exclusion_sizes(state: State<'_, AppState>) -> CmdResult<Vec<ExclusionSize>> {
    let paths = lock(&state.settings).exclusions.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let cancel = CancelToken::default();
        std::thread::scope(|s| {
            let jobs: Vec<_> = paths
                .iter()
                .map(|p| {
                    let cancel = &cancel;
                    s.spawn(move || {
                        let stats = crp_core::fsutil::measure_path(p, cancel);
                        ExclusionSize {
                            path: p.clone(),
                            exists: p.exists(),
                            bytes: stats.bytes,
                            items: stats.items,
                        }
                    })
                })
                .collect();
            jobs.into_iter().filter_map(|j| j.join().ok()).collect()
        })
    })
    .await
    .map_err(err)
}

/// Ruta de un hallazgo del último análisis (para copiarla o excluirla).
#[tauri::command]
pub fn finding_path(state: State<'_, AppState>, id: String) -> Option<PathBuf> {
    lock(&state.last_scan)
        .iter()
        .find(|f| f.id == id)
        .and_then(|f| f.path.clone())
}

const REPO: &str = "https://github.com/CristopherPaiz/RCleaner";

/// Abre en el navegador una página del proyecto. Sólo destinos conocidos:
/// `repo`, `issues`, `releases`, `license`, `contributors`, `user:<login>`.
#[tauri::command]
pub fn open_link(app: AppHandle, target: String) -> CmdResult<()> {
    let url = match target.as_str() {
        "repo" => REPO.to_owned(),
        "issues" => format!("{REPO}/issues"),
        "new_issue" => format!("{REPO}/issues/new/choose"),
        "releases" => format!("{REPO}/releases"),
        "license" => format!("{REPO}/blob/main/LICENSE"),
        "contributors" => format!("{REPO}/graphs/contributors"),
        "changelog" => format!("{REPO}/blob/main/CHANGELOG.md"),
        other => match other.strip_prefix("user:") {
            Some(login)
                if !login.is_empty()
                    && login.len() <= 39
                    && login.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') =>
            {
                format!("https://github.com/{login}")
            }
            _ => return Err(t("app.unknown_link")),
        },
    };
    app.opener().open_url(url, None::<&str>).map_err(err)
}

/// Cierra la ventana y deja RCleaner en la bandeja.
#[tauri::command]
pub fn to_tray(app: AppHandle) {
    crate::tray::hide(&app);
}

/// Relanza la app como administrador y cierra esta instancia. La nueva espera
/// a que esta termine (si no, la instancia única la cerraría).
#[tauri::command]
pub fn restart_as_admin(app: AppHandle) -> CmdResult<()> {
    let args = [crate::WAIT_ARG.to_owned(), std::process::id().to_string()];
    crp_platform::relaunch_elevated(&args)
        .map_err(|e| tf("app.elevate_failed", &[("error", &e)]))?;
    app.exit(0);
    Ok(())
}
