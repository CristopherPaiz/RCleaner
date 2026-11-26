//! Modos: perfiles de limpieza (rápido, programador, IA…) definidos en TOML.
//!
//! Un modo es una lista de **selectores** de módulos:
//! - `system.user_temp` → ese módulo.
//! - `dev.*`            → todos los módulos cuyo id empieza por `dev.`.
//! - `*`                → todos.
//! - `!dev.projects`    → excluye (se aplica después de las inclusiones).

use crate::error::{Error, Result};
use crate::i18n::{self, t, tf};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mode {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icon: String,
    pub modules: Vec<String>,
    /// Creado por el usuario (se puede editar/borrar).
    #[serde(default)]
    pub custom: bool,
}

#[derive(Serialize, Deserialize)]
struct ModesFile {
    #[serde(rename = "mode", default)]
    modes: Vec<Mode>,
}

const BUILTIN: &str = include_str!("../modes/builtin.toml");

/// Modos incluidos en el binario.
/// Modos incluidos en el binario, con nombre y descripción en el idioma actual
/// (`mode.<id>.name` / `mode.<id>.desc` en `locales/`; si no, lo del TOML).
pub fn builtin() -> Vec<Mode> {
    let mut modes = parse(BUILTIN).expect("modes/builtin.toml debe ser válido");
    for m in &mut modes {
        if let Some(n) = i18n::get(&format!("mode.{}.name", m.id)) {
            m.name = n;
        }
        if let Some(d) = i18n::get(&format!("mode.{}.desc", m.id)) {
            m.description = d;
        }
    }
    modes
}

/// Interpreta un archivo TOML con uno o más `[[mode]]`.
pub fn parse(toml_src: &str) -> Result<Vec<Mode>> {
    toml::from_str::<ModesFile>(toml_src)
        .map(|f| f.modes)
        .map_err(|e| Error::Config(e.to_string()))
}

/// Carga modos de usuario desde `*.toml` en `dir`. Los archivos inválidos se
/// devuelven como error junto a su nombre para mostrarlos en la UI.
pub fn load_dir(dir: &Path) -> (Vec<Mode>, Vec<String>) {
    let mut modes = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (modes, errors);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("toml"))
        {
            continue;
        }
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| parse(&s).map_err(|e| e.to_string()))
        {
            Ok(m) => modes.extend(m),
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    (modes, errors)
}

/// Id válido para un modo de usuario (también es su nombre de archivo).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Modos incluidos + modos del usuario en `user_dir`. Un modo de usuario no
/// puede reemplazar uno incluido (se informa como error).
pub fn all(user_dir: &Path) -> (Vec<Mode>, Vec<String>) {
    let mut modes = builtin();
    let (user, mut errors) = load_dir(user_dir);
    for mut m in user {
        if modes.iter().any(|b| b.id == m.id) {
            errors.push(tf("core.mode_exists", &[("id", &m.id)]));
            continue;
        }
        m.custom = true;
        modes.push(m);
    }
    (modes, errors)
}

/// Guarda (o reemplaza) un modo de usuario en `user_dir\<id>.toml`.
pub fn save_user(user_dir: &Path, mode: &Mode) -> Result<std::path::PathBuf> {
    if !valid_id(&mode.id) {
        return Err(Error::Config(t("core.mode_bad_id")));
    }
    if builtin().iter().any(|b| b.id == mode.id) {
        return Err(Error::Config(tf("core.mode_builtin", &[("id", &mode.id)])));
    }
    if mode.name.trim().is_empty() || mode.modules.is_empty() {
        return Err(Error::Config(t("core.mode_incomplete")));
    }
    let file = ModesFile {
        modes: vec![Mode {
            custom: true,
            ..mode.clone()
        }],
    };
    let src = toml::to_string(&file).map_err(|e| Error::Config(e.to_string()))?;
    std::fs::create_dir_all(user_dir).map_err(|e| Error::io(user_dir, e))?;
    let path = user_dir.join(format!("{}.toml", mode.id));
    std::fs::write(&path, src).map_err(|e| Error::io(&path, e))?;
    Ok(path)
}

/// Borra un modo de usuario.
pub fn delete_user(user_dir: &Path, id: &str) -> Result<()> {
    if !valid_id(id) {
        return Err(Error::Config(t("core.mode_bad_id")));
    }
    let path = user_dir.join(format!("{id}.toml"));
    std::fs::remove_file(&path).map_err(|e| Error::io(&path, e))
}

/// `true` si el selector positivo `selector` incluye el módulo `id`.
pub fn selector_matches(selector: &str, id: &str) -> bool {
    if selector == "*" || selector == id {
        return true;
    }
    selector
        .strip_suffix('*')
        .is_some_and(|prefix| prefix.ends_with('.') && id.starts_with(prefix))
}

/// Aplica una lista de selectores (con exclusiones `!`) a un id.
pub fn selected(selectors: &[String], id: &str) -> bool {
    let (excl, incl): (Vec<&String>, Vec<&String>) =
        selectors.iter().partition(|s| s.starts_with('!'));
    incl.iter().any(|s| selector_matches(s, id))
        && !excl.iter().any(|s| selector_matches(&s[1..], id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_modes_parse() {
        let modes = builtin();
        assert!(modes.iter().any(|m| m.id == "programador"));
        let mut ids: Vec<_> = modes.iter().map(|m| &m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), modes.len(), "ids de modo únicos");
    }

    #[test]
    fn user_modes_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let m = Mode {
            id: "mi-modo".into(),
            name: "Mi modo".into(),
            description: "d".into(),
            icon: String::new(),
            modules: vec!["dev.npm".into(), "system.*".into()],
            custom: false,
        };
        save_user(dir.path(), &m).unwrap();
        let (all_modes, errors) = all(dir.path());
        assert!(errors.is_empty(), "{errors:?}");
        let loaded = all_modes.iter().find(|x| x.id == "mi-modo").unwrap();
        assert!(loaded.custom);
        assert_eq!(loaded.modules, m.modules);

        assert!(
            save_user(
                dir.path(),
                &Mode {
                    id: "rapido".into(),
                    ..m.clone()
                }
            )
            .is_err()
        );
        assert!(
            save_user(
                dir.path(),
                &Mode {
                    id: "..\\x".into(),
                    ..m.clone()
                }
            )
            .is_err()
        );
        delete_user(dir.path(), "mi-modo").unwrap();
        assert!(!all(dir.path()).0.iter().any(|x| x.id == "mi-modo"));
    }

    #[test]
    fn selectors() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(selected(&s(&["dev.*"]), "dev.npm"));
        assert!(!selected(&s(&["dev.*"]), "developer.x"));
        assert!(selected(&s(&["*", "!ai.*"]), "dev.npm"));
        assert!(!selected(&s(&["*", "!ai.*"]), "ai.ollama"));
        assert!(!selected(&s(&["dev.*", "!dev.projects"]), "dev.projects"));
    }
}
