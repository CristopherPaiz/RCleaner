//! El propio proceso: memoria y espera a otro proceso.

use std::time::Duration;

/// Devuelve a Windows la memoria que el proceso no está usando (working set).
/// Útil al quedarse en la bandeja: lo que no se toque vuelve a cargarse solo.
#[cfg(windows)]
pub fn trim_memory() {
    use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    // SAFETY: GetCurrentProcess devuelve un pseudo-handle siempre válido que no
    // hay que cerrar; EmptyWorkingSet sólo lo usa durante la llamada.
    unsafe {
        K32EmptyWorkingSet(GetCurrentProcess());
    }
}

#[cfg(not(windows))]
pub fn trim_memory() {}

/// Espera a que termine el proceso `pid` (como mucho `timeout`). `true` si
/// terminó o ya no existe.
#[cfg(windows)]
pub fn wait_for_exit(pid: u32, timeout: Duration) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
    // SAFETY: OpenProcess no toca memoria nuestra; si devuelve un handle, se
    // espera sobre él y se cierra una sola vez.
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if h.is_null() {
            return true;
        }
        let r = WaitForSingleObject(h, ms);
        CloseHandle(h);
        r == WAIT_OBJECT_0
    }
}

#[cfg(not(windows))]
pub fn wait_for_exit(_pid: u32, _timeout: Duration) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_and_wait_do_not_fail() {
        trim_memory();
        // Un pid que no existe cuenta como terminado.
        assert!(wait_for_exit(u32::MAX - 3, Duration::from_millis(10)));
    }
}
