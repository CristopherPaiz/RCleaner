//! Caché de navegadores. Cookies e historial sólo si el usuario activa las
//! opciones de privacidad, y nunca premarcados. Contraseñas, marcadores,
//! extensiones y ajustes NO se tocan nunca.

use crp_core::rules::glob_under;
use crp_core::{Category, Engine, ModuleMeta, PathRuleModule, Risk, Target, t, tf};
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
/// Cookies de cada perfil (las versiones nuevas las guardan en `Network`).
const COOKIES: &[&str] = &[
    r"Network\Cookies",
    r"Network\Cookies-journal",
    "Cookies",
    "Cookies-journal",
];
/// Historial de cada perfil. Marcadores (`Bookmarks`), contraseñas
/// (`Login Data`) y formularios (`Web Data`) quedan fuera.
const HISTORY: &[&str] = &[
    "History",
    "History-journal",
    "Visited Links",
    "Top Sites",
    "Top Sites-journal",
    "Shortcuts",
    "Shortcuts-journal",
];

pub fn register(e: &mut Engine) {
    for &(id, name, rel, exe) in CHROMIUM {
        e.register(PathRuleModule::new(
            ModuleMeta::new(id, Category::Browser)
                .named(name)
                .describe(t("module.browser.chromium.desc"))
                .closes(&[exe]),
            move |ctx| chromium_targets(name, &ctx.env.local_app_data.join(rel), ctx.privacy),
        ));
    }

    e.register(PathRuleModule::new(
        ModuleMeta::new("browser.firefox", Category::Browser).closes(&["firefox.exe"]),
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
            let mut targets = vec![Target::contents("cache", roots)];
            if ctx.privacy {
                // En Firefox el historial comparte archivo con los marcadores
                // (places.sqlite): sólo se ofrecen las cookies.
                let cookies: Vec<PathBuf> = glob_under(
                    &ctx.env.roaming_app_data.join(r"Mozilla\Firefox\Profiles"),
                    "*",
                )
                .iter()
                .flat_map(|p| {
                    ["cookies.sqlite", "cookies.sqlite-wal", "cookies.sqlite-shm"]
                        .map(|f| p.join(f))
                })
                .collect();
                targets.push(cookies_target("Firefox", cookies));
            }
            targets
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

/// Cookies: datos del usuario (cierran sesiones), nunca premarcados.
fn cookies_target(name: &str, paths: Vec<PathBuf>) -> Target {
    Target::paths("cookies", paths)
        .label(tf("module.browser.privacy.cookies", &[("name", &name)]))
        .risk(Risk::Advanced)
        .detail(t("module.browser.privacy.cookies_detail"))
}

fn chromium_targets(name: &str, user_data: &Path, privacy: bool) -> Vec<Target> {
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

    let mut targets = vec![
        Target::contents("cache", cache)
            .label(tf("module.browser.chromium.cache", &[("name", &name)])),
        Target::contents("service_workers", sw)
            .label(tf(
                "module.browser.chromium.service_workers",
                &[("name", &name)],
            ))
            .risk(Risk::Moderate)
            .detail(t("module.browser.chromium.service_workers_detail")),
    ];
    if privacy {
        let files = |names: &'static [&'static str]| -> Vec<PathBuf> {
            profiles
                .iter()
                .flat_map(|p| names.iter().map(move |f| p.join(f)))
                .collect()
        };
        targets.push(cookies_target(name, files(COOKIES)));
        targets.push(
            Target::paths("history", files(HISTORY))
                .label(tf("module.browser.privacy.history", &[("name", &name)]))
                .risk(Risk::Advanced)
                .detail(t("module.browser.privacy.history_detail")),
        );
    }
    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn privacy_targets_only_when_enabled_and_never_touch_passwords() {
        let d = tempfile::tempdir().unwrap();
        let ud = d.path();
        let prof = ud.join("Default");
        fs::create_dir_all(prof.join("Network")).unwrap();
        fs::create_dir_all(prof.join("Cache")).unwrap();
        for f in ["History", "Login Data", "Bookmarks", r"Network\Cookies"] {
            fs::write(prof.join(f), b"x").unwrap();
        }
        let keys = |privacy| {
            chromium_targets("X", ud, privacy)
                .into_iter()
                .map(|t| t.key.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(false), ["cache", "service_workers"]);
        assert_eq!(
            keys(true),
            ["cache", "service_workers", "cookies", "history"]
        );
        for t in chromium_targets("X", ud, true) {
            for r in &t.roots {
                let n = r.file_name().unwrap().to_string_lossy();
                assert!(
                    n != "Login Data" && n != "Bookmarks" && n != "Web Data",
                    "{n}"
                );
            }
        }
    }
}
