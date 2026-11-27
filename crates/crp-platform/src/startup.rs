//! Programas que arrancan con Windows: claves `Run` del registro y carpetas
//! Inicio. Activar/desactivar usa lo mismo que el Administrador de tareas
//! (`Explorer\StartupApproved`): no se borra ni se mueve nada, así que
//! siempre se puede deshacer.

use serde::Serialize;
use std::io;
use std::path::PathBuf;

/// Para quién arranca.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Sólo el usuario actual (no necesita administrador).
    User,
    /// Todos los usuarios (cambiarlo necesita administrador).
    Machine,
}

/// De dónde sale la entrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// `…\CurrentVersion\Run`
    Run,
    /// `WOW6432Node\…\Run` (programas de 32 bits, sólo del equipo).
    Run32,
    /// Acceso directo en una carpeta Inicio.
    Folder,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupEntry {
    /// Nombre del valor del registro o del archivo de la carpeta.
    pub name: String,
    /// Lo que ejecuta Windows (para la carpeta, la ruta del acceso directo).
    pub command: String,
    /// Ejecutable (o acceso directo), para el icono y para abrir su carpeta.
    pub file: Option<PathBuf>,
    pub scope: Scope,
    pub source: Source,
    pub enabled: bool,
}

impl StartupEntry {
    /// Identificador estable: `scope/source/name`.
    pub fn id(&self) -> String {
        format!("{:?}/{:?}/{}", self.scope, self.source, self.name).to_lowercase()
    }
}

/// Ejecutable de una línea de comandos (`"C:\a b\x.exe" --min` → `C:\a b\x.exe`).
pub fn command_exe(command: &str) -> Option<PathBuf> {
    let c = command.trim();
    let raw = if let Some(rest) = c.strip_prefix('"') {
        rest.split('"').next()?.to_owned()
    } else {
        let lower = c.to_ascii_lowercase();
        match lower.find(".exe") {
            Some(i) => c[..i + 4].to_owned(),
            None => c.split_whitespace().next()?.to_owned(),
        }
    };
    let expanded = expand_env(&raw);
    (!expanded.is_empty()).then(|| PathBuf::from(expanded))
}

/// Sustituye `%VAR%` por su valor (si existe).
fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) if !name.is_empty() => out.push_str(&v),
                    _ => out.push_str(&rest[start..start + end + 2]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// ¿Habilitada según el valor de `StartupApproved`? (byte 0 par = sí).
pub fn approved(data: Option<&[u8]>) -> bool {
    data.and_then(|d| d.first()).is_none_or(|b| b & 1 == 0)
}

/// Valor de `StartupApproved` para habilitar/deshabilitar (como el
/// Administrador de tareas: 12 bytes; al deshabilitar, la fecha).
pub fn approved_value(enabled: bool, filetime: u64) -> Vec<u8> {
    let mut v = vec![if enabled { 2 } else { 3 }, 0, 0, 0];
    v.extend_from_slice(&if enabled { 0 } else { filetime }.to_le_bytes());
    v
}

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, RegType};
    use winreg::{RegKey, RegValue};

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

    fn hive(scope: Scope) -> RegKey {
        RegKey::predef(match scope {
            Scope::User => HKEY_CURRENT_USER,
            Scope::Machine => HKEY_LOCAL_MACHINE,
        })
    }

    fn approved_key(source: Source) -> String {
        let sub = match source {
            Source::Run => "Run",
            Source::Run32 => "Run32",
            Source::Folder => "StartupFolder",
        };
        format!(r"{APPROVED}\{sub}")
    }

    fn is_enabled(scope: Scope, source: Source, name: &str) -> bool {
        let data = hive(scope)
            .open_subkey_with_flags(approved_key(source), KEY_READ)
            .and_then(|k| k.get_raw_value(name))
            .ok();
        approved(data.as_ref().map(|v| v.bytes.as_ref()))
    }

    fn folder(scope: Scope) -> Option<PathBuf> {
        let base = match scope {
            Scope::User => std::env::var_os("APPDATA")?,
            Scope::Machine => std::env::var_os("ProgramData")?,
        };
        Some(PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
    }

    pub fn list() -> Vec<StartupEntry> {
        let mut out = Vec::new();
        for (scope, source, path) in [
            (Scope::User, Source::Run, RUN),
            (Scope::Machine, Source::Run, RUN),
            (Scope::Machine, Source::Run32, RUN32),
        ] {
            let Ok(key) = hive(scope).open_subkey_with_flags(path, KEY_READ) else {
                continue;
            };
            for (name, value) in key.enum_values().flatten() {
                if !matches!(value.vtype, RegType::REG_SZ | RegType::REG_EXPAND_SZ) {
                    continue;
                }
                let command = value.to_string();
                if command.trim().is_empty() {
                    continue;
                }
                out.push(StartupEntry {
                    enabled: is_enabled(scope, source, &name),
                    file: command_exe(&command),
                    name,
                    command,
                    scope,
                    source,
                });
            }
        }
        for scope in [Scope::User, Scope::Machine] {
            let Some(dir) = folder(scope) else { continue };
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.eq_ignore_ascii_case("desktop.ini") || !e.path().is_file() {
                    continue;
                }
                out.push(StartupEntry {
                    enabled: is_enabled(scope, Source::Folder, &name),
                    command: e.path().to_string_lossy().into_owned(),
                    file: Some(e.path()),
                    name,
                    scope,
                    source: Source::Folder,
                });
            }
        }
        out
    }

    pub fn set_enabled(scope: Scope, source: Source, name: &str, enabled: bool) -> io::Result<()> {
        let (key, _) = hive(scope).create_subkey_with_flags(approved_key(source), KEY_SET_VALUE)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() / 100)
            .unwrap_or(0) as u64;
        // FILETIME cuenta desde 1601.
        const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
        let value = RegValue {
            bytes: approved_value(enabled, now + EPOCH_DIFF).into(),
            vtype: RegType::REG_BINARY,
        };
        key.set_raw_value(name, &value)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn list() -> Vec<StartupEntry> {
        Vec::new()
    }
    pub fn set_enabled(_: Scope, _: Source, _: &str, _: bool) -> io::Result<()> {
        Err(io::Error::other("unsupported"))
    }
}

/// Todo lo que arranca al iniciar sesión (habilitado o no).
pub fn list() -> Vec<StartupEntry> {
    imp::list()
}

/// Habilita o deshabilita una entrada (`Machine` necesita administrador).
pub fn set_enabled(scope: Scope, source: Source, name: &str, enabled: bool) -> io::Result<()> {
    imp::set_enabled(scope, source, name, enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_from_command_lines() {
        assert_eq!(
            command_exe(r#""C:\Program Files\App\app.exe" --min"#),
            Some(PathBuf::from(r"C:\Program Files\App\app.exe"))
        );
        assert_eq!(
            command_exe(r"C:\Tools\x.EXE /background"),
            Some(PathBuf::from(r"C:\Tools\x.EXE"))
        );
        assert_eq!(command_exe("  "), None);
    }

    #[test]
    fn startup_approved_flags() {
        assert!(approved(None));
        assert!(approved(Some(&[2, 0, 0, 0])));
        assert!(approved(Some(&[6, 0])));
        assert!(!approved(Some(&[3, 0, 0, 0])));
        let on = approved_value(true, 42);
        let off = approved_value(false, 42);
        assert_eq!(on.len(), 12);
        assert!(approved(Some(&on)));
        assert!(!approved(Some(&off)));
        assert_eq!(u64::from_le_bytes(off[4..].try_into().unwrap()), 42);
    }
}
