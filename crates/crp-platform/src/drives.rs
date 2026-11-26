//! Unidades fijas y su espacio libre.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveInfo {
    /// `C:\`
    pub root: String,
    /// Etiqueta del volumen (puede estar vacía).
    pub label: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

/// Discos locales (no extraíbles, no de red).
#[cfg(windows)]
pub fn fixed_drives() -> Vec<DriveInfo> {
    use crate::{from_wide, wide};
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };
    const DRIVE_FIXED: u32 = 3;

    let mut drives = Vec::new();
    // SAFETY: sin parámetros.
    let mask = unsafe { GetLogicalDrives() };
    for i in 0..26u8 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let root = format!("{}:\\", (b'A' + i) as char);
        let w = wide(&root);
        // SAFETY: `w` es UTF-16 terminado en nulo; los punteros de salida son válidos.
        unsafe {
            if GetDriveTypeW(w.as_ptr()) != DRIVE_FIXED {
                continue;
            }
            let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
            if GetDiskFreeSpaceExW(w.as_ptr(), &mut avail, &mut total, &mut free) == 0 {
                continue;
            }
            let mut label = [0u16; 261];
            let ok = GetVolumeInformationW(
                w.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
            let label = if ok != 0 {
                from_wide(&label)
            } else {
                String::new()
            };
            drives.push(DriveInfo {
                root,
                label,
                total_bytes: total,
                free_bytes: free,
            });
        }
    }
    drives
}

#[cfg(not(windows))]
pub fn fixed_drives() -> Vec<DriveInfo> {
    Vec::new()
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn system_drive_is_listed() {
        let drives = super::fixed_drives();
        let sys = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let d = drives
            .iter()
            .find(|d| d.root.starts_with(&sys))
            .expect("unidad del sistema");
        assert!(d.total_bytes > 0 && d.free_bytes <= d.total_bytes);
    }
}
