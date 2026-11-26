//! Ajustes del usuario e historial de limpiezas, en `%APPDATA%\RCleaner`.
//!
//! ```text
//! %APPDATA%\RCleaner\
//!   settings.json   carpetas de proyectos, exclusiones, modo por defecto
//!   history.json    últimas limpiezas (fecha, modo, espacio liberado)
//!   modes\*.toml    modos personalizados
//! ```

use crate::env::Env;
use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Máximo de entradas guardadas en el historial.
const HISTORY_MAX: usize = 500;

/// Ubicación de los archivos de la app.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
}

impl AppPaths {
    pub fn new(env: &Env) -> Self {
        let config_dir = env.roaming_app_data.join("RCleaner");
        // La app se llamaba Clean-CRP: conservar sus ajustes, historial y modos.
        let legacy = env.roaming_app_data.join("CleanCRP");
        if legacy.is_dir() && !config_dir.exists() {
            let _ = std::fs::rename(&legacy, &config_dir);
        }
        Self { config_dir }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn history_file(&self) -> PathBuf {
        self.config_dir.join("history.json")
    }

    pub fn modes_dir(&self) -> PathBuf {
        self.config_dir.join("modes")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Carpetas de proyectos. Vacío = detección automática.
    pub dev_roots: Vec<PathBuf>,
    /// Rutas que nunca se limpian (ni lo que hay dentro).
    pub exclusions: Vec<PathBuf>,
    /// Modo seleccionado al abrir la app.
    pub default_mode: Option<String>,
    /// Código de idioma (`es`, `en`…). `None` = el del sistema.
    pub language: Option<String>,
    /// `light` / `dark`. `None` = el del sistema.
    pub theme: Option<String>,
}

impl Settings {
    /// Lee los ajustes; si el archivo no existe devuelve los valores por defecto.
    pub fn load(path: &Path) -> Result<Self> {
        read_json(path).map(Option::unwrap_or_default)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        write_json(path, self)
    }
}

/// Resumen de una limpieza.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Segundos desde 1970 (UTC).
    pub timestamp: u64,
    pub mode: Option<String>,
    pub freed_bytes: u64,
    pub findings: usize,
    pub skipped_items: u64,
}

impl HistoryEntry {
    pub fn now(
        mode: Option<String>,
        freed_bytes: u64,
        findings: usize,
        skipped_items: u64,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self {
            timestamp,
            mode,
            freed_bytes,
            findings,
            skipped_items,
        }
    }
}

/// Historial (más reciente al final). Archivo ausente o dañado = vacío.
pub fn load_history(path: &Path) -> Vec<HistoryEntry> {
    read_json(path).ok().flatten().unwrap_or_default()
}

pub fn append_history(path: &Path, entry: HistoryEntry) -> Result<()> {
    let mut all = load_history(path);
    all.push(entry);
    let excess = all.len().saturating_sub(HISTORY_MAX);
    all.drain(..excess);
    write_json(path, &all)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| Error::Config(format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// Escritura atómica: archivo temporal + renombrado.
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    }
    let json = serde_json::to_vec_pretty(value).map_err(|e| Error::Config(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| Error::io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(r"RCleaner\settings.json");
        assert_eq!(Settings::load(&file).unwrap(), Settings::default());

        let s = Settings {
            dev_roots: vec![PathBuf::from(r"D:\code")],
            exclusions: vec![PathBuf::from(r"C:\keep")],
            default_mode: Some("programador".into()),
            language: Some("en".into()),
            theme: None,
        };
        s.save(&file).unwrap();
        assert_eq!(Settings::load(&file).unwrap(), s);
    }

    #[test]
    fn corrupt_settings_is_error_history_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x.json");
        std::fs::write(&f, "{no json").unwrap();
        assert!(Settings::load(&f).is_err());
        assert!(load_history(&f).is_empty());
    }

    #[test]
    fn history_is_capped() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("history.json");
        for i in 0..(HISTORY_MAX + 5) {
            append_history(&f, HistoryEntry::now(None, i as u64, 1, 0)).unwrap();
        }
        let h = load_history(&f);
        assert_eq!(h.len(), HISTORY_MAX);
        assert_eq!(h.last().unwrap().freed_bytes, (HISTORY_MAX + 4) as u64);
    }
}
