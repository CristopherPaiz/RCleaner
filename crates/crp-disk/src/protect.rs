//! Qué no se puede eliminar desde Espacio o Duplicados, y por qué.
//!
//! El analizador enseña **todo** el disco, incluidos archivos que Windows
//! necesita (`hiberfil.sys`, `pagefile.sys`, el registro del usuario, la
//! papelera, puntos de restauración…). Se muestran para entender el espacio,
//! pero no se pueden mandar a la papelera: la UI los marca con un candado y el
//! backend los rechaza otra vez al borrar.

use crp_core::{Env, path_within, safety, t};
use std::path::{Component, Path};

/// Atributo Win32 `FILE_ATTRIBUTE_SYSTEM`.
pub const SYSTEM_ATTRIBUTE: u32 = 0x4;

/// Motivo (traducido) por el que `path` no se puede eliminar, o `None` si se puede.
///
/// `system` = el archivo o carpeta tiene el atributo de sistema.
pub fn reason(path: &Path, system: bool, env: &Env) -> Option<String> {
    reason_key(path, system, env).map(|k| t(&format!("protect.{k}")))
}

fn reason_key(path: &Path, system: bool, env: &Env) -> Option<&'static str> {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .collect();

    if let Some(k) = parts.last().and_then(|n| by_name(n)) {
        return Some(k);
    }
    if let Some(k) = parts
        .first()
        .and_then(|top| by_top_folder(top, parts.len()))
    {
        return Some(k);
    }
    let installed = [
        &env.windows_dir,
        &env.program_files,
        &env.program_files_x86,
        &env.program_data,
    ];
    if installed.iter().any(|d| path_within(path, d)) {
        return Some("installed");
    }
    if parts.len() < 2 {
        return Some("drive_root");
    }
    let app_bases = [
        env.local_app_data.join("Packages"),
        env.local_app_data.join("Microsoft"),
        env.local_app_data.join("Programs"),
        env.roaming_app_data.join("Microsoft"),
    ];
    let is_profile = env
        .home
        .parent()
        .is_some_and(|users| path.parent().is_some_and(|p| same(p, users)));
    if is_profile
        || app_bases.iter().any(|b| same(path, b))
        || safety::check_target(path, env).is_err()
    {
        return Some("base_folder");
    }
    if system {
        return Some("system_file");
    }
    None
}

fn same(a: &Path, b: &Path) -> bool {
    path_within(a, b) && path_within(b, a)
}

/// Archivos que Windows usa estén donde estén.
fn by_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "hiberfil.sys" => "hiberfil",
        "pagefile.sys" => "pagefile",
        "swapfile.sys" => "swapfile",
        "bootmgr" | "bootnxt" | "bootsect.bak" => "boot",
        "dumpstack.log" | "dumpstack.log.tmp" => "system_file",
        n if n.starts_with("ntuser.") || n.starts_with("usrclass.dat") => "registry",
        _ => return None,
    })
}

/// Carpetas de primer nivel de una unidad que gestiona Windows.
fn by_top_folder(top: &str, depth: usize) -> Option<&'static str> {
    Some(match top {
        "$recycle.bin" => "recycle_bin",
        "system volume information" => "restore",
        "windows.old" | "$windows.~bt" | "$windows.~ws" | "$winreagent" | "$sysreset"
        | "$getcurrent" => "windows_old",
        "recovery" | "boot" | "efi" => "boot",
        "windows"
        | "program files"
        | "program files (x86)"
        | "programdata"
        | "config.msi"
        | "msocache" => "installed",
        "users" if depth <= 2 => "base_folder",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn key(p: &str, system: bool) -> Option<&'static str> {
        let env = Env::sandbox(Path::new(r"C:\sb"));
        reason_key(&PathBuf::from(p), system, &env)
    }

    #[test]
    fn windows_files_are_protected_anywhere() {
        assert_eq!(key(r"C:\hiberfil.sys", true), Some("hiberfil"));
        assert_eq!(key(r"D:\pagefile.sys", false), Some("pagefile"));
        assert_eq!(key(r"C:\swapfile.sys", true), Some("swapfile"));
        assert_eq!(key(r"C:\sb\home\NTUSER.DAT", false), Some("registry"));
        assert_eq!(
            key(
                r"C:\sb\home\AppData\Local\Microsoft\Windows\UsrClass.dat",
                false
            ),
            Some("registry")
        );
    }

    #[test]
    fn windows_folders_are_protected() {
        assert_eq!(
            key(r"C:\$Recycle.Bin\S-1-5-21\$R1.zip", false),
            Some("recycle_bin")
        );
        assert_eq!(
            key(r"E:\System Volume Information\x", false),
            Some("restore")
        );
        assert_eq!(key(r"C:\Windows.old\Users", false), Some("windows_old"));
        assert_eq!(key(r"C:\Windows\Installer\a.msi", false), Some("installed"));
        assert_eq!(
            key(r"C:\Program Files\App\big.dat", false),
            Some("installed")
        );
        assert_eq!(key(r"C:\Users\Someone", false), Some("base_folder"));
        assert_eq!(key(r"D:\Games", false), Some("drive_root"));
        assert_eq!(
            key(r"C:\sb\home\AppData\Local\Packages", false),
            Some("base_folder")
        );
        assert_eq!(
            key(r"C:\sb\home\Documents\desktop.ini", true),
            Some("system_file")
        );
    }

    #[test]
    fn user_files_are_allowed() {
        assert_eq!(key(r"C:\sb\home\Downloads\setup.exe", false), None);
        assert_eq!(key(r"D:\Games\old\game.iso", false), None);
        assert_eq!(
            key(r"C:\sb\home\AppData\Local\Packages\App\LocalCache", false),
            None
        );
        assert_eq!(key(r"C:\Users\Someone\Videos\a.mp4", false), None);
    }
}
