//! Papelera de reciclaje de todas las unidades.

use std::io;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecycleBinInfo {
    pub bytes: u64,
    pub items: u64,
}

#[cfg(windows)]
mod imp {
    use super::RecycleBinInfo;
    use std::io;
    use windows_sys::Win32::UI::Shell::{
        SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinW, SHQUERYRBINFO,
        SHQueryRecycleBinW,
    };

    fn check(hr: i32) -> io::Result<()> {
        if hr >= 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(hr))
        }
    }

    pub fn query() -> io::Result<RecycleBinInfo> {
        let mut info = SHQUERYRBINFO {
            cbSize: size_of::<SHQUERYRBINFO>() as u32,
            i64Size: 0,
            i64NumItems: 0,
        };
        // SAFETY: raíz nula = todas las unidades; `info` es válido y con cbSize correcto.
        check(unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) })?;
        Ok(RecycleBinInfo {
            bytes: info.i64Size.max(0) as u64,
            items: info.i64NumItems.max(0) as u64,
        })
    }

    pub fn empty() -> io::Result<()> {
        if query()?.items == 0 {
            return Ok(());
        }
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        // SAFETY: sin ventana propietaria, raíz nula = todas las unidades.
        check(unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), flags) })
    }
}

#[cfg(not(windows))]
mod imp {
    use super::RecycleBinInfo;
    use std::io;

    pub fn query() -> io::Result<RecycleBinInfo> {
        Ok(RecycleBinInfo::default())
    }

    pub fn empty() -> io::Result<()> {
        Ok(())
    }
}

/// Tamaño y número de elementos en la papelera (todas las unidades).
pub fn query() -> io::Result<RecycleBinInfo> {
    imp::query()
}

/// Vacía la papelera sin diálogos. No hace nada si ya está vacía.
pub fn empty() -> io::Result<()> {
    imp::empty()
}
