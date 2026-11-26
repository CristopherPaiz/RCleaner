//! Ajustes del usuario e historial de limpiezas, en `%APPDATA%\RCleaner`.
//!
//! ```text
//! %APPDATA%\RCleaner\
//!   settings.json   carpetas de proyectos, exclusiones, modo por defecto
//!   history.json    últimas limpiezas (fecha, modo, espacio liberado)
//!   deleted.log     registro de todo lo eliminado (una línea JSON por ruta)
//!   modes\*.toml    modos personalizados
//! ```

use crate::env::Env;
use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Máximo de entradas guardadas en el historial.
const HISTORY_MAX: usize = 500;
/// Tamaño a partir del cual el registro de borrados se recorta.
const DELETIONS_MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Líneas que se conservan al recortar el registro de borrados.
const DELETIONS_KEEP: usize = 10_000;

/// Segundos desde 1970 (UTC).
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

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

    pub fn deletions_file(&self) -> PathBuf {
        self.config_dir.join("deleted.log")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Abrir siempre como administrador (Windows lo pide al iniciar).
    pub always_admin: bool,
    /// Al minimizar, ocultar la ventana en la bandeja del sistema.
    pub tray: bool,
    /// Iniciar con Windows (en la bandeja).
    pub autostart: bool,
    /// Análisis periódico en segundo plano.
    pub background: Background,
    /// Permitir elegir archivos de Windows protegidos en Espacio y
    /// Duplicados. Cada uno pide confirmar que el usuario asume el riesgo.
    pub allow_system: bool,
    /// Ya se completó el asistente de la primera vez.
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dev_roots: Vec::new(),
            exclusions: Vec::new(),
            default_mode: None,
            language: None,
            theme: None,
            always_admin: false,
            tray: true,
            autostart: false,
            background: Background::default(),
            allow_system: false,
            onboarded: false,
        }
    }
}

/// Análisis periódico en segundo plano: si lo que se puede limpiar supera el
/// umbral, avisa con una notificación de Windows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Background {
    pub enabled: bool,
    /// Horas entre análisis (1–168).
    pub every_hours: u32,
    /// Avisar a partir de estos MB limpiables.
    pub threshold_mb: u64,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            enabled: false,
            every_hours: 6,
            threshold_mb: 1024,
        }
    }
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
        Self {
            timestamp: unix_now(),
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

/// Algo que se eliminó (o se mandó a la papelera).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletionRecord {
    /// Segundos desde 1970 (UTC).
    pub timestamp: u64,
    /// `clean`, `space` o `dupes`.
    pub source: String,
    pub path: PathBuf,
    pub bytes: u64,
    /// `true` = borrado definitivo; `false` = a la papelera.
    pub permanent: bool,
}

/// Añade registros al final de `deleted.log` (una línea JSON cada uno).
/// Si el archivo crece demasiado, se quedan las últimas líneas.
pub fn append_deletions(path: &Path, records: &[DeletionRecord]) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    }
    let mut out = Vec::new();
    for r in records {
        serde_json::to_writer(&mut out, r).map_err(|e| Error::Config(e.to_string()))?;
        out.push(b'\n');
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| Error::io(path, e))?;
    file.write_all(&out).map_err(|e| Error::io(path, e))?;
    drop(file);

    if std::fs::metadata(path).is_ok_and(|m| m.len() > DELETIONS_MAX_BYTES) {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        let lines: Vec<&str> = text.lines().collect();
        let keep = &lines[lines.len().saturating_sub(DELETIONS_KEEP)..];
        let tmp = path.with_extension("log.tmp");
        std::fs::write(&tmp, keep.join("\n") + "\n").map_err(|e| Error::io(&tmp, e))?;
        std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))?;
    }
    Ok(())
}

/// Los últimos `limit` registros, el más reciente primero. Las líneas
/// dañadas se ignoran; sin archivo, vacío.
pub fn load_deletions(path: &Path, limit: usize) -> Vec<DeletionRecord> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .rev()
        .filter_map(|l| serde_json::from_str(l).ok())
        .take(limit)
        .collect()
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
            always_admin: true,
            background: Background {
                enabled: true,
                ..Background::default()
            },
            ..Settings::default()
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

    #[test]
    fn old_settings_get_new_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("settings.json");
        std::fs::write(&f, r#"{"theme":"dark"}"#).unwrap();
        let s = Settings::load(&f).unwrap();
        assert_eq!(s.theme.as_deref(), Some("dark"));
        assert!(s.tray);
        assert!(!s.allow_system);
        assert_eq!(s.background.every_hours, 6);
    }

    #[test]
    fn deletions_log_appends_and_reads_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join(r"RCleaner\deleted.log");
        assert!(load_deletions(&f, 10).is_empty());
        let rec = |n: u64| DeletionRecord {
            timestamp: n,
            source: "space".into(),
            path: PathBuf::from(format!(r"C:\x\{n}")),
            bytes: n,
            permanent: false,
        };
        append_deletions(&f, &[rec(1), rec(2)]).unwrap();
        append_deletions(&f, &[rec(3)]).unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&f)
            .unwrap()
            .write_all(b"{broken\n")
            .unwrap();
        let got = load_deletions(&f, 2);
        assert_eq!(got, vec![rec(3), rec(2)]);
    }
}
