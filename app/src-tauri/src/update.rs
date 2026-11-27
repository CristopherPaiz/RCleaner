//! Actualizaciones: `latest.json` de la última release de GitHub, firmada con
//! la llave cuya parte pública va en `tauri.conf.json`.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

type CmdResult<T> = Result<T, String>;

#[derive(Serialize)]
pub struct UpdateInfo {
    version: String,
    notes: Option<String>,
    /// `false` en la versión portable (y en desarrollo): se ofrece descargar.
    installable: bool,
}

#[derive(Clone, Serialize)]
struct UpdateProgress {
    done: u64,
    total: Option<u64>,
}

/// Instalada con el setup (deja `uninstall.exe` junto al exe; las versiones
/// antiguas con MSI, en Program Files). La portable no se reemplaza a sí
/// misma: abrirá la página de descargas.
fn installed() -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    exe.parent()
        .is_some_and(|d| d.join("uninstall.exe").is_file())
        || ["ProgramFiles", "ProgramW6432"]
            .iter()
            .filter_map(std::env::var_os)
            .any(|pf| exe.starts_with(pf))
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> CmdResult<Option<UpdateInfo>> {
    let update = match app.updater().map_err(|e| e.to_string())?.check().await {
        Ok(u) => u,
        // La última release no trae `latest.json` (publicada sin firmar):
        // no hay nada que ofrecer, no es un error de conexión.
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => None,
        Err(e) => return Err(e.to_string()),
    };
    Ok(update.map(|u| UpdateInfo {
        version: u.version.clone(),
        notes: u.body.clone(),
        installable: installed(),
    }))
}

/// Descarga, verifica la firma e instala. El instalador cierra RCleaner y lo
/// vuelve a abrir al terminar.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> CmdResult<()> {
    if !installed() {
        return Err("portable".into());
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no update".to_owned())?;
    let mut done = 0u64;
    let events = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let _ = events.emit("crp://update", UpdateProgress { done, total });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}
