//! Caché de navegadores. Sólo caché: cookies, historial y contraseñas NO se tocan.

use crp_core::rules::glob_under;
use crp_core::{Category, Engine, ModuleMeta, PathRuleModule, Risk, Target};
use std::path::{Path, PathBuf};

/// Navegadores basados en Chromium: (id, nombre, carpeta bajo %LOCALAPPDATA%, ejecutable).
const CHROMIUM: &[(&str, &str, &str, &str)] = &[
    (
        "browser.chrome",
        "Google Chrome",
        r"Google\Chrome\User Data",
        "chrome.exe",
    ),
    (
        "browser.edge",
        "Microsoft Edge",
        r"Microsoft\Edge\User Data",
        "msedge.exe",
    ),
    (
        "browser.brave",
        "Brave",
        r"BraveSoftware\Brave-Browser\User Data",
        "brave.exe",
    ),
    (
        "browser.vivaldi",
        "Vivaldi",
        r"Vivaldi\User Data",
        "vivaldi.exe",
    ),
    (
        "browser.chromium",
        "Chromium",
        r"Chromium\User Data",
        "chrome.exe",
    ),
    // Opera guarda la caché directamente en la carpeta del perfil.
    (
        "browser.opera",
        "Opera",
        r"Opera Software\Opera Stable",
        "opera.exe",
    ),
    (
        "browser.opera_gx",
        "Opera GX",
        r"Opera Software\Opera GX Stable",
        "opera.exe",
    ),
];

/// Cachés dentro de cada perfil.
const PROFILE_CACHES: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
];
/// Cachés de Service Workers (datos offline de webs/PWA).
const SW_CACHES: &[&str] = &[
    r"Service Worker\CacheStorage",
    r"Service Worker\ScriptCache",
];
/// Cachés a nivel de "User Data".
const BASE_CACHES: &[&str] = &["ShaderCache", "GrShaderCache", "GraphiteDawnCache"];

pub fn register(e: &mut Engine) {
    for &(id, name, rel, exe) in CHROMIUM {
        e.register(PathRuleModule::new(
            ModuleMeta::new(id, name, Category::Browser)
                .describe("Caché web, de código y de GPU de todos los perfiles.")
                .closes(&[exe]),
            move |ctx| chromium_targets(name, &ctx.env.local_app_data.join(rel)),
        ));
    }

    e.register(PathRuleModule::new(
        ModuleMeta::new("browser.firefox", "Mozilla Firefox", Category::Browser)
            .describe("Caché web (cache2), startupCache y miniaturas de todos los perfiles.")
            .closes(&["firefox.exe"]),
        |ctx| {
            let profiles = glob_under(
                &ctx.env.local_app_data.join(r"Mozilla\Firefox\Profiles"),
                "*",
            );
            let roots: Vec<PathBuf> = profiles
                .iter()
                .flat_map(|p| {
                    ["cache2", "startupCache", "thumbnails", "jumpListCache"].map(|s| p.join(s))
                })
                .collect();
            vec![Target::contents("cache", "Caché de Firefox", roots)]
        },
    ));
}

/// Carpetas de perfil: `Default`, `Profile N`, `Guest Profile`… o la base misma (Opera).
fn chromium_profiles(user_data: &Path) -> Vec<PathBuf> {
    let mut profiles = vec![user_data.to_path_buf()];
    for pattern in ["Default", "Profile *", "Guest Profile", "System Profile"] {
        profiles.extend(glob_under(user_data, pattern));
    }
    profiles
}

fn chromium_targets(name: &str, user_data: &Path) -> Vec<Target> {
    if !user_data.is_dir() {
        return Vec::new();
    }
    let profiles = chromium_profiles(user_data);
    let mut cache: Vec<PathBuf> = BASE_CACHES.iter().map(|c| user_data.join(c)).collect();
    cache.extend(
        profiles
            .iter()
            .flat_map(|p| PROFILE_CACHES.iter().map(move |c| p.join(c))),
    );
    let sw: Vec<PathBuf> = profiles
        .iter()
        .flat_map(|p| SW_CACHES.iter().map(move |c| p.join(c)))
        .collect();

    vec![
        Target::contents("cache", &format!("Caché de {name}"), cache),
        Target::contents("service_workers", &format!("Service Workers de {name}"), sw)
            .risk(Risk::Moderate)
            .detail("Datos offline de sitios y PWAs; se vuelven a descargar al visitarlos."),
    ]
}
