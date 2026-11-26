//! Permisos de administrador.

use std::io;

/// `true` si el proceso actual corre como administrador.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    // SAFETY: función sin parámetros que sólo consulta el token del proceso.
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Relanza el ejecutable actual como administrador (diálogo UAC).
///
/// Devuelve error si el usuario cancela el UAC. Quien llama debe cerrar el
/// proceso actual si esto tiene éxito.
#[cfg(windows)]
pub fn relaunch_elevated(args: &[String]) -> io::Result<()> {
    use crate::wide;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    const SW_SHOWNORMAL: i32 = 1;

    let exe = std::env::current_exe()?;
    let params = args
        .iter()
        .map(|a| {
            if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let (verb, file, params) = (wide("runas"), wide(&exe), wide(&params));
    // SAFETY: todas las cadenas son UTF-16 terminadas en nulo y viven durante la llamada.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute devuelve un valor > 32 si tuvo éxito.
    if result as isize > 32 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
pub fn relaunch_elevated(_args: &[String]) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "sólo Windows"))
}
