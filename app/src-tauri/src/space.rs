//! Comandos del analizador de espacio y del buscador de duplicados.
//!
//! Igual que en la limpieza, la UI sólo envía **ids** de nodos/archivos del
//! último análisis; las rutas las resuelve el backend. Todo lo que se elimina
//! va a la papelera de reciclaje y pasa antes por las guardas de seguridad.

use crate::state::{AppState, lock};
use crp_core::{Env, path_within, safety, t, tf};
use crp_disk::dupes::{DupeOptions, DupeProgress, DupeReport, find};
use crp_disk::protect;
use crp_disk::tree::{
    Crumb, Entry, FileEntry, NodeId, ScanOptions, ScanProgress, Summary, Treemap, ViewNode, scan,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

const DISK_EVENT: &str = "crp://disk";
const DUPES_EVENT: &str = "crp://dupes";

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize, Default)]
pub struct DeleteResult {
    freed_bytes: u64,
    removed: usize,
    errors: Vec<String>,
}

/// Archivos y carpetas de Windows (`crp_disk::protect`), `safety::check_target`
/// y nada dentro de Windows, Program Files o ProgramData: eso se quita con el
/// desinstalador, no a mano. `system` = tiene el atributo de sistema.
fn deletable(path: &Path, system: bool, env: &Env) -> Result<(), String> {
    if let Some(why) = protect::reason(path, system, env) {
        return Err(format!("{}: {why}", path.display()));
    }
    safety::check_target(path, env).map_err(err)?;
    let system = [
        &env.windows_dir,
        &env.program_files,
        &env.program_files_x86,
        &env.program_data,
    ];
    if system.iter().any(|s| path_within(path, s)) {
        return Err(tf("app.system_path", &[("path", &path.display())]));
    }
    Ok(())
}

fn is_system(md: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    md.file_attributes() & protect::SYSTEM_ATTRIBUTE != 0
}

/// Manda a la papelera; si el lote falla, reintenta de uno en uno para saber cuáles.
fn recycle(paths: &[PathBuf]) -> Vec<(PathBuf, Result<(), String>)> {
    if crp_platform::recycle_bin::send(paths).is_ok() {
        return paths.iter().map(|p| (p.clone(), Ok(()))).collect();
    }
    paths
        .iter()
        .map(|p| {
            let r = if !p.exists() {
                Ok(()) // ya se movió en el intento por lotes
            } else {
                crp_platform::recycle_bin::send(std::slice::from_ref(p))
                    .map_err(|e| format!("{}: {e}", p.display()))
            };
            (p.clone(), r)
        })
        .collect()
}

/// Abre la carpeta, o la carpeta que contiene el archivo con el archivo
/// seleccionado. `explorer.exe` no entiende `"/select,ruta"` entre comillas
/// (lo que hace `Command::arg` si hay espacios) y abre "Documentos", así que
/// el argumento se pasa tal cual: `/select,"ruta"`.
pub fn open_in_explorer(path: &Path) -> CmdResult<()> {
    use std::os::windows::process::CommandExt;
    let quoted = format!("\"{}\"", path.display());
    let arg = if path.is_dir() {
        quoted
    } else {
        format!("/select,{quoted}")
    };
    std::process::Command::new("explorer.exe")
        .raw_arg(arg)
        .spawn()
        .map_err(err)?;
    Ok(())
}

/// Emite el progreso cada 150 ms mientras `running` sea verdadero.
fn ticker(app: AppHandle, progress: Arc<ScanProgress>, running: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        while running.load(Ordering::Relaxed) {
            let _ = app.emit(DISK_EVENT, progress.snapshot());
            std::thread::sleep(Duration::from_millis(150));
        }
    });
}

// ── Analizador ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn disk_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    path: PathBuf,
) -> CmdResult<Summary> {
    if !path.is_absolute() {
        return Err(t("app.bad_path"));
    }
    let cancel = AppState::fresh_token(&state.disk_cancel);
    let opts = ScanOptions {
        exclusions: lock(&state.settings).exclusions.clone(),
    };
    let progress = Arc::new(ScanProgress::default());
    let running = Arc::new(AtomicBool::new(true));
    ticker(app, progress.clone(), running.clone());

    let result =
        tauri::async_runtime::spawn_blocking(move || scan(&path, &opts, &cancel, &progress)).await;
    running.store(false, Ordering::Relaxed);

    let tree = result.map_err(err)?.map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => t("app.no_permission"),
        _ => e.to_string(),
    })?;
    let summary = tree.summary().clone();
    *lock(&state.disk) = Some(tree);
    Ok(summary)
}

/// Resumen del último análisis (para volver a la vista sin repetirlo).
#[tauri::command]
pub fn disk_summary(state: State<'_, AppState>) -> Option<Summary> {
    lock(&state.disk).as_ref().map(|t| t.summary().clone())
}

#[tauri::command]
pub fn disk_view(state: State<'_, AppState>, id: NodeId, depth: u32) -> Option<ViewNode> {
    lock(&state.disk)
        .as_ref()
        .and_then(|t| t.view(id, depth.clamp(1, 8), 0.004))
}

/// Mapa de rectángulos bajo `id` (hasta archivos; lo diminuto, agrupado).
#[tauri::command]
pub fn disk_treemap(state: State<'_, AppState>, id: NodeId) -> Option<Treemap> {
    lock(&state.disk)
        .as_ref()
        .and_then(|t| t.treemap(id, 50_000, 1.0 / 400_000.0))
}

#[tauri::command]
pub fn disk_children(state: State<'_, AppState>, id: NodeId) -> Vec<Entry> {
    lock(&state.disk)
        .as_ref()
        .map(|t| t.children(id, 500))
        .unwrap_or_default()
}

#[tauri::command]
pub fn disk_crumbs(state: State<'_, AppState>, id: NodeId) -> Vec<Crumb> {
    lock(&state.disk)
        .as_ref()
        .map(|t| t.ancestors(id))
        .unwrap_or_default()
}

#[tauri::command]
pub fn disk_largest(state: State<'_, AppState>, id: NodeId) -> Vec<FileEntry> {
    lock(&state.disk)
        .as_ref()
        .map(|t| t.largest_files(id, 200))
        .unwrap_or_default()
}

#[tauri::command]
pub fn disk_reveal(state: State<'_, AppState>, id: NodeId) -> CmdResult<()> {
    let path = lock(&state.disk)
        .as_ref()
        .and_then(|t| t.path(id))
        .ok_or_else(|| t("app.unknown_item"))?;
    open_in_explorer(&path)
}

#[tauri::command]
pub async fn disk_delete(state: State<'_, AppState>, ids: Vec<NodeId>) -> CmdResult<DeleteResult> {
    let env = Env::from_system();
    let mut result = DeleteResult::default();
    let mut targets: Vec<(NodeId, PathBuf)> = Vec::new();
    {
        let guard = lock(&state.disk);
        let tree = guard.as_ref().ok_or_else(|| t("app.no_analysis"))?;
        for id in ids {
            let Some(path) = tree.path(id).filter(|_| tree.is_attached(id)) else {
                result.errors.push(t("app.item_gone"));
                continue;
            };
            let system = tree.node(id).is_some_and(|n| n.system);
            match deletable(&path, system, &env) {
                Ok(()) => targets.push((id, path)),
                Err(e) => result.errors.push(e),
            }
        }
    }
    // Si se eligieron una carpeta y algo dentro de ella, basta con la carpeta.
    let all: Vec<PathBuf> = targets.iter().map(|(_, p)| p.clone()).collect();
    targets.retain(|(_, p)| !all.iter().any(|o| o != p && path_within(p, o)));

    let paths: Vec<PathBuf> = targets.iter().map(|(_, p)| p.clone()).collect();
    let outcomes = tauri::async_runtime::spawn_blocking(move || recycle(&paths))
        .await
        .map_err(err)?;

    let mut guard = lock(&state.disk);
    let tree = guard.as_mut().ok_or_else(|| t("app.analysis_dropped"))?;
    for ((id, _), (path, r)) in targets.into_iter().zip(outcomes) {
        match r {
            Ok(()) if !path.exists() => {
                result.freed_bytes += tree.remove(id).unwrap_or(0);
                result.removed += 1;
            }
            Ok(()) => result
                .errors
                .push(format!("{}: no se pudo mover", path.display())),
            Err(e) => result.errors.push(e),
        }
    }
    Ok(result)
}

// ── Duplicados ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn dupes_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    roots: Vec<PathBuf>,
    min_size: u64,
    include_system: bool,
) -> CmdResult<DupeReport> {
    let roots: Vec<PathBuf> = roots
        .into_iter()
        .filter(|r| r.is_absolute() && r.is_dir())
        .collect();
    if roots.is_empty() {
        return Err(t("app.pick_existing_folder"));
    }
    let mut opts = DupeOptions::new(roots);
    opts.min_size = min_size.max(1);
    opts.exclusions = lock(&state.settings).exclusions.clone();
    if include_system {
        opts.skip_dirs.clear();
    }
    let cancel = AppState::fresh_token(&state.dupes_cancel);
    let report = tauri::async_runtime::spawn_blocking(move || {
        let progress = move |p: DupeProgress| {
            let _ = app.emit(DUPES_EVENT, p);
        };
        let rep = find(&opts, &cancel, &progress);
        (!cancel.is_cancelled()).then_some(rep)
    })
    .await
    .map_err(err)?
    .ok_or_else(|| t("app.search_cancelled"))?;

    *lock(&state.dupes) = Some(report.clone());
    Ok(report)
}

#[tauri::command]
pub fn dupes_reveal(state: State<'_, AppState>, id: u32) -> CmdResult<()> {
    let path = lock(&state.dupes)
        .as_ref()
        .and_then(|r| r.find(id).map(|(_, f)| f.path.clone()))
        .ok_or_else(|| t("app.unknown_file"))?;
    open_in_explorer(&path)
}

#[tauri::command]
pub async fn dupes_delete(state: State<'_, AppState>, ids: Vec<u32>) -> CmdResult<DeleteResult> {
    let env = Env::from_system();
    let wanted: HashSet<u32> = ids.into_iter().collect();
    let mut result = DeleteResult::default();
    let mut targets: Vec<(u32, PathBuf, u64)> = Vec::new();
    {
        let guard = lock(&state.dupes);
        let rep = guard.as_ref().ok_or_else(|| t("app.no_search"))?;
        // Nunca borrar todas las copias de un grupo.
        let mut per_group: HashMap<usize, usize> = HashMap::new();
        for &id in &wanted {
            let (g, f) = rep.find(id).ok_or_else(|| t("app.file_gone"))?;
            *per_group.entry(g).or_default() += 1;
            let system = std::fs::symlink_metadata(&f.path).is_ok_and(|m| is_system(&m));
            match deletable(&f.path, system, &env) {
                Ok(()) => targets.push((id, f.path.clone(), rep.groups[g].size)),
                Err(e) => result.errors.push(e),
            }
        }
        if per_group
            .iter()
            .any(|(&g, &n)| n >= rep.groups[g].files.len())
        {
            return Err(t("app.keep_one_copy"));
        }
    }

    let paths: Vec<PathBuf> = targets.iter().map(|(_, p, _)| p.clone()).collect();
    let outcomes = tauri::async_runtime::spawn_blocking(move || recycle(&paths))
        .await
        .map_err(err)?;

    let mut removed = HashSet::new();
    for ((id, _, size), (path, r)) in targets.into_iter().zip(outcomes) {
        match r {
            Ok(()) if !path.exists() => {
                removed.insert(id);
                result.freed_bytes += size;
                result.removed += 1;
            }
            Ok(()) => result
                .errors
                .push(format!("{}: no se pudo mover", path.display())),
            Err(e) => result.errors.push(e),
        }
    }
    if let Some(rep) = lock(&state.dupes).as_mut() {
        rep.forget(&removed);
    }
    Ok(result)
}

/// Último resultado de duplicados (tras borrar, ya sin lo eliminado).
#[tauri::command]
pub fn dupes_report(state: State<'_, AppState>) -> Option<DupeReport> {
    lock(&state.dupes).clone()
}
