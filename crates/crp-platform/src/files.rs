//! Identidad de archivos (detección de enlaces duros) y atributos de nube.

use std::path::Path;

/// Identifica un archivo dentro del sistema: dos rutas con la misma identidad
/// son el mismo contenido en disco (enlaces duros). Borrar una no libera espacio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub volume: u32,
    pub index: u64,
    pub links: u32,
}

/// Atributos de archivos "a petición" (OneDrive, Google Drive…): el contenido
/// no está en disco y leerlo provocaría una descarga.
pub const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
pub const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x40000;
pub const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

/// `true` si los atributos indican un marcador de la nube sin contenido local.
pub fn is_cloud_placeholder(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

#[cfg(windows)]
pub fn identity(path: &Path) -> Option<FileIdentity> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
        OPEN_EXISTING,
    };

    let wide = crate::wide(path.as_os_str());
    // SAFETY: `wide` termina en nulo; sólo pedimos leer atributos (no abre el
    // contenido, así que no dispara descargas de la nube).
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    // SAFETY: estructura POD inicializada a cero; `handle` es válido.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) } != 0;
    // SAFETY: cerramos el handle que abrimos arriba.
    unsafe { CloseHandle(handle) };
    ok.then(|| FileIdentity {
        volume: info.dwVolumeSerialNumber,
        index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        links: info.nNumberOfLinks,
    })
}

#[cfg(not(windows))]
pub fn identity(_path: &Path) -> Option<FileIdentity> {
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn hard_links_share_identity() {
        let dir = std::env::temp_dir().join(format!("crp-id-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        let c = dir.join("c.bin");
        std::fs::write(&a, b"hola").unwrap();
        std::fs::write(&c, b"hola").unwrap();
        let _ = std::fs::remove_file(&b);
        std::fs::hard_link(&a, &b).unwrap();

        let ia = identity(&a).unwrap();
        assert_eq!(ia, identity(&b).unwrap());
        assert_eq!(ia.links, 2);
        assert_ne!(ia, identity(&c).unwrap());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn placeholder_flags() {
        assert!(is_cloud_placeholder(
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS | 0x20
        ));
        assert!(!is_cloud_placeholder(0x20));
    }
}
