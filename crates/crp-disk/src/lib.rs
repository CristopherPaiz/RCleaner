//! # crp-disk
//!
//! Herramientas de espacio que **no** son limpiadores por reglas:
//!
//! - [`tree`]: analizador de uso de disco (mapa radial tipo DaisyDisk /
//!   WinDirStat). Recorre una unidad o carpeta en paralelo y guarda un árbol
//!   compacto en memoria que la UI consulta por niveles.
//! - [`dupes`]: buscador de archivos duplicados por contenido
//!   (tamaño → hash parcial → hash completo, BLAKE3), sin contar enlaces duros
//!   ni descargar archivos de la nube.
//!
//! - [`protect`]: qué no se puede eliminar (archivos y carpetas de Windows).
//!
//! Nada aquí borra por sí mismo: la eliminación va a la papelera
//! (`crp_platform::recycle_bin::send`) y pasa por `crp_core::safety`.
// Todo el código de sistema (`unsafe`/FFI) vive sólo en crp-platform.
#![forbid(unsafe_code)]

pub mod dupes;
pub mod protect;
pub mod tree;

use std::time::{SystemTime, UNIX_EPOCH};

/// Segundos Unix de una fecha de archivo (0 si no se conoce).
pub(crate) fn unix_secs(t: std::io::Result<SystemTime>) -> u64 {
    t.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

/// Atributos Win32 del archivo (0 fuera de Windows).
pub(crate) fn attributes(md: &std::fs::Metadata) -> u32 {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        md.file_attributes()
    }
    #[cfg(not(windows))]
    {
        let _ = md;
        0
    }
}
