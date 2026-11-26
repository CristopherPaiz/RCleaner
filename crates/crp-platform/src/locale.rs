//! Idioma de la interfaz de Windows.

/// Código BCP-47 del idioma de la interfaz del usuario (`es-MX`, `en-US`).
#[cfg(windows)]
pub fn ui_language() -> Option<String> {
    use windows_sys::Win32::Globalization::{GetUserDefaultUILanguage, LCIDToLocaleName};
    let mut buf = [0u16; 85];
    // SAFETY: búfer local con el tamaño indicado (LOCALE_NAME_MAX_LENGTH = 85).
    let n = unsafe {
        let lang = u32::from(GetUserDefaultUILanguage());
        LCIDToLocaleName(lang, buf.as_mut_ptr(), buf.len() as i32, 0)
    };
    (n > 0).then(|| crate::from_wide(&buf))
}

#[cfg(not(windows))]
pub fn ui_language() -> Option<String> {
    std::env::var("LANG")
        .ok()
        .map(|l| l.split('.').next().unwrap_or_default().to_owned())
}
