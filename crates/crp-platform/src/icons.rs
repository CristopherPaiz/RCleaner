//! Iconos que Windows muestra en el Explorador (lo mismo que hace WinDirStat).
//!
//! No se incluye ningún icono de terceros: se le piden a Windows en el PC del
//! usuario. Si algo falla se devuelve `None` y la UI usa sus propios iconos.

use std::path::PathBuf;

/// Qué icono pedir.
#[derive(Debug, Clone)]
pub enum IconQuery {
    /// El de un tipo de archivo (`"zip"`, `"pdf"`, `""` = sin extensión).
    /// No toca el disco.
    Extension(String),
    /// El de un archivo concreto (`.exe`, `.lnk`, `.ico` traen el suyo).
    File(PathBuf),
    /// Carpeta genérica.
    Folder,
    /// Unidad (`C:\`).
    Drive(PathBuf),
}

/// PNG (32×32, con transparencia) del icono, o `None` si Windows no da uno.
///
/// Inicializa COM en el hilo que llama: úsese desde un hilo de trabajo.
pub fn png(query: &IconQuery) -> Option<Vec<u8>> {
    imp::png(query)
}

/// Ruta del ejecutable de una app instalada (`"chrome.exe"`), según el
/// registro de Windows (`App Paths`) y las carpetas por usuario habituales.
pub fn find_app(exe: &str) -> Option<PathBuf> {
    imp::app_path(exe).or_else(|| per_user_app(exe))
}

/// Apps que se instalan por usuario sin registrarse en `App Paths`.
fn per_user_app(exe: &str) -> Option<PathBuf> {
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let rel: &[&str] = match exe.to_ascii_lowercase().as_str() {
        "opera.exe" => &[r"Programs\Opera\opera.exe", r"Programs\Opera GX\opera.exe"],
        "code.exe" => &[r"Programs\Microsoft VS Code\Code.exe"],
        "cursor.exe" => &[r"Programs\cursor\Cursor.exe"],
        "windsurf.exe" => &[r"Programs\Windsurf\Windsurf.exe"],
        "ollama app.exe" | "ollama.exe" => &[r"Programs\Ollama\ollama app.exe"],
        "lm studio.exe" => &[r"Programs\LM Studio\LM Studio.exe"],
        "brave.exe" => &[r"BraveSoftware\Brave-Browser\Application\brave.exe"],
        "vivaldi.exe" => &[r"Vivaldi\Application\vivaldi.exe"],
        "chrome.exe" => &[r"Google\Chrome\Application\chrome.exe"],
        _ => &[],
    };
    rel.iter().map(|r| local.join(r)).find(|p| p.is_file())
}

#[cfg(windows)]
mod imp {
    use super::IconQuery;
    use crate::{from_wide, wide};
    use std::ffi::c_void;
    use std::path::PathBuf;
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC,
        GetDIBits, GetObjectW, ReleaseDC,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    };
    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
    };
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW,
    };
    use windows_sys::Win32::UI::Shell::{
        SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    pub fn png(query: &IconQuery) -> Option<Vec<u8>> {
        // SAFETY: COM por hilo; se deshace sólo si este intento lo inicializó.
        let com = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
        let out = shell_icon(query).and_then(|h| {
            let rgba = icon_rgba(h);
            // SAFETY: icono devuelto por SHGetFileInfoW; lo liberamos una vez.
            unsafe { DestroyIcon(h) };
            rgba
        });
        if com >= 0 {
            // SAFETY: equilibra el CoInitializeEx que tuvo éxito.
            unsafe { CoUninitialize() };
        }
        let (w, h, rgba) = out?;
        encode(w, h, &rgba)
    }

    fn shell_icon(query: &IconQuery) -> Option<HICON> {
        let (path, attrs, extra) = match query {
            IconQuery::Extension(ext) => {
                let name = if ext.is_empty() {
                    "file".to_owned()
                } else {
                    format!("file.{ext}")
                };
                (wide(name), FILE_ATTRIBUTE_NORMAL, SHGFI_USEFILEATTRIBUTES)
            }
            IconQuery::Folder => (
                wide("folder"),
                FILE_ATTRIBUTE_DIRECTORY,
                SHGFI_USEFILEATTRIBUTES,
            ),
            IconQuery::File(p) | IconQuery::Drive(p) => (wide(p), 0, 0),
        };
        // SAFETY: estructura cero-inicializada que la API rellena.
        let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
        // SAFETY: `path` es UTF-16 con nulo final y vive durante la llamada.
        let ok = unsafe {
            SHGetFileInfoW(
                path.as_ptr(),
                attrs,
                &mut info,
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON | extra,
            )
        };
        (ok != 0 && !info.hIcon.is_null()).then_some(info.hIcon)
    }

    /// Píxeles RGBA de un icono (con la máscara si no trae canal alfa).
    fn icon_rgba(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
        // SAFETY: estructura que GetIconInfo rellena; sus mapas de bits se
        // liberan siempre antes de salir.
        let mut ii: ICONINFO = unsafe { std::mem::zeroed() };
        if unsafe { GetIconInfo(icon, &mut ii) } == 0 {
            return None;
        }
        let result = (|| {
            let color = ii.hbmColor;
            if color.is_null() {
                return None; // iconos monocromo: no merecen la pena
            }
            let mut bm: BITMAP = unsafe { std::mem::zeroed() };
            // SAFETY: `bm` tiene el tamaño que se indica.
            if unsafe {
                GetObjectW(
                    color,
                    size_of::<BITMAP>() as i32,
                    (&mut bm as *mut BITMAP).cast(),
                )
            } == 0
            {
                return None;
            }
            let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
            if w <= 0 || h <= 0 || w > 256 || h > 256 {
                return None;
            }
            let mut px = dib(color, w, h)?;
            if px.chunks_exact(4).all(|p| p[3] == 0) {
                // Sin alfa: la máscara dice qué es transparente (blanco).
                let mask = if ii.hbmMask.is_null() {
                    None
                } else {
                    dib(ii.hbmMask, w, h)
                };
                for (i, p) in px.chunks_exact_mut(4).enumerate() {
                    let transparent = mask.as_ref().is_some_and(|m| m[i * 4] != 0);
                    p[3] = if transparent { 0 } else { 255 };
                }
            }
            // BGRA → RGBA
            for p in px.chunks_exact_mut(4) {
                p.swap(0, 2);
            }
            Some((w as u32, h as u32, px))
        })();
        // SAFETY: mapas de bits creados por GetIconInfo; nos pertenecen.
        unsafe {
            if !ii.hbmColor.is_null() {
                DeleteObject(ii.hbmColor);
            }
            if !ii.hbmMask.is_null() {
                DeleteObject(ii.hbmMask);
            }
        }
        result
    }

    /// Mapa de bits como BGRA de 32 bits, de arriba abajo.
    fn dib(bmp: *mut c_void, w: i32, h: i32) -> Option<Vec<u8>> {
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB as _,
            ..unsafe { std::mem::zeroed() }
        };
        let mut px = vec![0u8; (w * h * 4) as usize];
        // SAFETY: DC de pantalla prestado y devuelto; `px` cabe w×h×4 bytes.
        let lines = unsafe {
            let dc = GetDC(std::ptr::null_mut());
            let n = GetDIBits(
                dc,
                bmp,
                0,
                h as u32,
                px.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            );
            ReleaseDC(std::ptr::null_mut(), dc);
            n
        };
        (lines == h).then_some(px)
    }

    fn encode(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
        writer.finish().ok()?;
        Some(out)
    }

    pub fn app_path(exe: &str) -> Option<PathBuf> {
        let key = wide(format!(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe}"
        ));
        for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            let mut buf = vec![0u16; 1024];
            let mut len = (buf.len() * 2) as u32;
            // SAFETY: `buf` tiene `len` bytes; valor por defecto (nombre nulo).
            let rc = unsafe {
                RegGetValueW(
                    root,
                    key.as_ptr(),
                    std::ptr::null(),
                    RRF_RT_REG_SZ,
                    std::ptr::null_mut(),
                    buf.as_mut_ptr().cast(),
                    &mut len,
                )
            };
            if rc == 0 {
                let s = from_wide(&buf);
                let p = PathBuf::from(s.trim().trim_matches('"'));
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        None
    }
}

#[cfg(not(windows))]
mod imp {
    use super::IconQuery;
    use std::path::PathBuf;

    pub fn png(_: &IconQuery) -> Option<Vec<u8>> {
        None
    }

    pub fn app_path(_: &str) -> Option<PathBuf> {
        None
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn folder_and_common_types_have_icons() {
        let folder = png(&IconQuery::Folder).expect("icono de carpeta");
        assert_eq!(&folder[1..4], b"PNG");
        assert!(png(&IconQuery::Extension("txt".into())).is_some());
        let explorer = PathBuf::from(std::env::var("WINDIR").unwrap()).join("explorer.exe");
        assert!(png(&IconQuery::File(explorer)).is_some());
    }
}
