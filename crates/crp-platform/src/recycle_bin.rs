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

    pub fn send(paths: &[std::path::PathBuf]) -> io::Result<()> {
        use windows_sys::Win32::UI::Shell::{
            FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT,
            FOF_WANTNUKEWARNING, SHFILEOPSTRUCTW, SHFileOperationW,
        };
        // Lista de rutas separadas por nulo y terminada en doble nulo.
        let mut from: Vec<u16> = Vec::new();
        for p in paths {
            from.extend(crate::wide(p.as_os_str()));
        }
        from.push(0);
        // FOF_WANTNUKEWARNING: si algo no cabe en la papelera, Windows pregunta
        // antes de borrarlo para siempre (anula FOF_NOCONFIRMATION sólo en ese caso).
        let flags =
            FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI | FOF_WANTNUKEWARNING;
        let mut op = SHFILEOPSTRUCTW {
            hwnd: std::ptr::null_mut(),
            wFunc: FO_DELETE,
            pFrom: from.as_ptr(),
            pTo: std::ptr::null(),
            fFlags: flags as u16,
            fAnyOperationsAborted: 0,
            hNameMappings: std::ptr::null_mut(),
            lpszProgressTitle: std::ptr::null(),
        };
        // SAFETY: `from` vive hasta el final de la llamada y está doble-terminado.
        let rc = unsafe { SHFileOperationW(&mut op) };
        if rc != 0 {
            return Err(io::Error::other(format!("SHFileOperation código {rc:#x}")));
        }
        if op.fAnyOperationsAborted != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "operación cancelada",
            ));
        }
        Ok(())
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

    pub fn send(paths: &[std::path::PathBuf]) -> io::Result<()> {
        for p in paths {
            if p.is_dir() {
                std::fs::remove_dir_all(p)?;
            } else {
                std::fs::remove_file(p)?;
            }
        }
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

/// Mueve archivos o carpetas a la papelera (se pueden restaurar).
/// Si algo no cabe en la papelera, Windows avisa antes de borrarlo.
pub fn send(paths: &[std::path::PathBuf]) -> io::Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    imp::send(paths)
}
