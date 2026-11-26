//! # crp-platform
//!
//! Única capa con `unsafe`/FFI. Envuelve llamadas Win32 en funciones seguras
//! para que el resto del proyecto no dependa de `windows-sys`.
//! En otras plataformas compila con implementaciones vacías (útil para CI/tests).

pub mod drives;
pub mod elevation;
pub mod files;
pub mod locale;
pub mod processes;
pub mod recycle_bin;

pub use elevation::{is_elevated, relaunch_elevated};

/// Convierte a UTF-16 terminado en nulo para la API Win32.
#[cfg(windows)]
pub(crate) fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// Texto UTF-16 hasta el primer nulo.
#[cfg(windows)]
pub(crate) fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
