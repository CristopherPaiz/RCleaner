//! Rutas conocidas del sistema, resueltas una vez y fáciles de simular en tests.

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Carpetas base del usuario/sistema más una instantánea de variables de entorno.
#[derive(Debug, Clone, Serialize)]
pub struct Env {
    /// `%USERPROFILE%`
    pub home: PathBuf,
    /// `%LOCALAPPDATA%`
    pub local_app_data: PathBuf,
    /// `%APPDATA%`
    pub roaming_app_data: PathBuf,
    /// `%TEMP%`
    pub temp: PathBuf,
    /// `%ProgramData%`
    pub program_data: PathBuf,
    /// `%SystemRoot%`
    pub windows_dir: PathBuf,
    /// `%ProgramFiles%`
    pub program_files: PathBuf,
    /// `%ProgramFiles(x86)%`
    pub program_files_x86: PathBuf,
    #[serde(skip)]
    vars: HashMap<String, String>,
}

impl Env {
    /// Lee las rutas del proceso actual.
    pub fn from_system() -> Self {
        let vars: HashMap<String, String> = std::env::vars()
            .map(|(k, v)| (k.to_ascii_uppercase(), v))
            .collect();
        let get = |k: &str| vars.get(k).filter(|v| !v.is_empty()).map(PathBuf::from);

        let home = dirs::home_dir()
            .or_else(|| get("USERPROFILE"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default"));
        let local_app_data = get("LOCALAPPDATA").unwrap_or_else(|| home.join(r"AppData\Local"));
        let roaming_app_data = get("APPDATA").unwrap_or_else(|| home.join(r"AppData\Roaming"));
        let windows_dir = get("SYSTEMROOT")
            .or_else(|| get("WINDIR"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let program_data = get("PROGRAMDATA").unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        let program_files =
            get("PROGRAMFILES").unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
        let program_files_x86 =
            get("PROGRAMFILES(X86)").unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));

        Self {
            temp: std::env::temp_dir(),
            home,
            local_app_data,
            roaming_app_data,
            program_data,
            windows_dir,
            program_files,
            program_files_x86,
            vars,
        }
    }

    /// Entorno falso con todo bajo `root` (para pruebas). Sin variables.
    pub fn sandbox(root: &Path) -> Self {
        let home = root.join("home");
        Self {
            local_app_data: home.join(r"AppData\Local"),
            roaming_app_data: home.join(r"AppData\Roaming"),
            temp: home.join(r"AppData\Local\Temp"),
            program_data: root.join("ProgramData"),
            windows_dir: root.join("Windows"),
            program_files: root.join("Program Files"),
            program_files_x86: root.join("Program Files (x86)"),
            home,
            vars: HashMap::new(),
        }
    }

    /// Define una variable (útil en pruebas).
    pub fn with_var(mut self, key: &str, value: impl Into<String>) -> Self {
        self.vars.insert(key.to_ascii_uppercase(), value.into());
        self
    }

    /// Variable de entorno no vacía (insensible a mayúsculas).
    pub fn var(&self, key: &str) -> Option<&str> {
        self.vars
            .get(&key.to_ascii_uppercase())
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    /// Variable de entorno interpretada como ruta.
    pub fn var_path(&self, key: &str) -> Option<PathBuf> {
        self.var(key).map(PathBuf::from)
    }

    /// `%USERPROFILE%\AppData\LocalLow`
    pub fn local_low(&self) -> PathBuf {
        self.home.join(r"AppData\LocalLow")
    }
}
