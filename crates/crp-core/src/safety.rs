//! Guardas que impiden limpiar rutas críticas aunque un módulo se equivoque.

use crate::env::Env;
use crate::error::{Error, Result};
use std::path::{Component, Path};

/// Normaliza para comparar en Windows: minúsculas, `\` y sin separador final.
fn norm(path: &Path) -> String {
    let s = path.to_string_lossy().replace('/', "\\").to_lowercase();
    s.trim_end_matches('\\').to_owned()
}

/// Falla si `path` no es un objetivo de limpieza aceptable.
///
/// Reglas:
/// - Debe ser absoluta y sin `..`.
/// - Al menos dos componentes bajo la raíz (`C:\a\b`); nunca `C:\` ni `C:\Windows`.
/// - No puede ser una carpeta base del usuario/sistema (perfil, AppData, Documentos…).
/// - No puede estar dentro de `System32`, `SysWOW64`, `WinSxS`, `servicing` o `Boot`.
pub fn check_target(path: &Path, env: &Env) -> Result<()> {
    let protected = || Err(Error::Protected(path.to_path_buf()));

    if !path.is_absolute() || path.components().any(|c| c == Component::ParentDir) {
        return protected();
    }
    let depth = path
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();
    if depth < 2 {
        return protected();
    }

    let target = norm(path);
    let home = &env.home;
    let exact = [
        home.clone(),
        home.join("AppData"),
        env.local_app_data.clone(),
        env.roaming_app_data.clone(),
        env.local_low(),
        home.join("Documents"),
        home.join("Desktop"),
        home.join("Downloads"),
        home.join("Pictures"),
        home.join("Music"),
        home.join("Videos"),
        home.join("OneDrive"),
        env.program_data.clone(),
        env.windows_dir.clone(),
        env.program_files.clone(),
        env.program_files_x86.clone(),
    ];
    if exact.iter().any(|p| norm(p) == target) {
        return protected();
    }

    let w = &env.windows_dir;
    let forbidden_trees = [
        w.join("System32"),
        w.join("SysWOW64"),
        w.join("WinSxS"),
        w.join("servicing"),
        w.join("Boot"),
    ];
    for tree in &forbidden_trees {
        let t = norm(tree);
        if target == t || target.starts_with(&format!("{t}\\")) {
            return protected();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn env() -> Env {
        Env::sandbox(Path::new(r"C:\sb"))
    }

    #[test]
    fn rejects_roots_and_bases() {
        let e = env();
        for p in [
            r"C:\",
            r"C:\sb",
            r"relative\path",
            r"C:\sb\home",
            r"C:\sb\home\AppData\Local",
        ] {
            assert!(
                check_target(&PathBuf::from(p), &e).is_err(),
                "{p} debería estar protegida"
            );
        }
        assert!(check_target(&e.windows_dir.join(r"System32\drivers"), &e).is_err());
        assert!(check_target(&PathBuf::from(r"C:\sb\home\..\home\x"), &e).is_err());
    }

    #[test]
    fn accepts_caches() {
        let e = env();
        assert!(check_target(&e.temp, &e).is_ok());
        assert!(check_target(&e.windows_dir.join("Temp"), &e).is_ok());
        assert!(check_target(&e.local_app_data.join(r"pip\Cache"), &e).is_ok());
    }

    #[test]
    fn case_and_trailing_separator_insensitive() {
        let e = env();
        assert!(check_target(&PathBuf::from(r"c:\SB\HOME\appdata\"), &e).is_err());
    }
}
