//! Procesos en ejecución (para avisar de navegadores/editores abiertos).

use std::collections::HashSet;

/// Nombres de ejecutable en marcha, en minúsculas (`chrome.exe`).
#[cfg(windows)]
pub fn running() -> HashSet<String> {
    use crate::from_wide;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    let mut names = HashSet::new();
    // SAFETY: instantánea de procesos; la estructura se inicializa con su tamaño
    // y el handle se cierra siempre.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return names;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                names.insert(from_wide(&entry.szExeFile).to_lowercase());
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    names
}

#[cfg(not(windows))]
pub fn running() -> HashSet<String> {
    HashSet::new()
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn sees_current_process() {
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_lowercase();
        assert!(super::running().contains(&name));
    }
}
