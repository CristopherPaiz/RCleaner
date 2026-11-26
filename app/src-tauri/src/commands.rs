//! Comandos expuestos al frontend. Todo el trabajo pesado corre en hilos de
//! bloqueo para no congelar la ventana; el progreso se emite como evento.

use crate::state::{AppState, lock};
use crp_core::context::default_dev_roots;
use crp_core::settings::{append_history, load_history};
use crp_core::{
    CleanOptions, CleanReport, Env, HistoryEntry, Mode, ModuleMeta, ProgressEvent, ScanReport,
    Settings, modes, t, tf,
};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

const PROGRESS_EVENT: &str = "crp://progress";

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
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> CmdResult<()> {
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
    };
    settings.save(&state.paths.settings_file()).map_err(err)?;
    let language_changed = lock(&state.settings).language != settings.language;
    if language_changed {
        crate::state::apply_language(&settings);
        // Los nombres de módulos y hallazgos se generan en el idioma activo.
        *lock(&state.engine) = crp_modules::engine();
        lock(&state.last_scan).clear();
    }
    *lock(&state.settings) = settings;
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

    let engine = lock(&state.engine).clone();
    let ctx = state.fresh_context();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let sink = move |e: ProgressEvent| {
            let _ = app.emit(PROGRESS_EVENT, e);
        };
        engine.clean(&findings, CleanOptions { dry_run }, &ctx, &sink)
    })
    .await
    .map_err(err)?;

    if !dry_run {
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
        || state.paths.config_dir == path;
    if !known {
        return Err(t("app.unknown_path"));
    }
    crate::space::open_in_explorer(&path)
}

/// Relanza la app como administrador y cierra esta instancia.
#[tauri::command]
pub fn restart_as_admin(app: AppHandle) -> CmdResult<()> {
    crp_platform::relaunch_elevated(&[]).map_err(|e| tf("app.elevate_failed", &[("error", &e)]))?;
    app.exit(0);
    Ok(())
}
