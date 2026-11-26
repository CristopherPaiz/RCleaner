//! Cachés globales de gestores de paquetes y editores.
//!
//! Cada caché respeta su variable de entorno (`PIP_CACHE_DIR`, `CARGO_HOME`…)
//! y si no existe usa la ruta por defecto en Windows.

use crp_core::rules::glob_under;
use crp_core::{Category, Engine, ModuleMeta, PathRuleModule, Risk, ScanContext, Target, tf};
use std::path::PathBuf;

/// `$VAR` si está definida, si no `default`.
fn env_or(ctx: &ScanContext, var: &str, default: PathBuf) -> PathBuf {
    ctx.env.var_path(var).unwrap_or(default)
}

fn dev(id: &str) -> ModuleMeta {
    ModuleMeta::new(id, Category::Dev)
}

pub fn register(e: &mut Engine) {
    // ── Python ──────────────────────────────────────────────────────────────
    e.register(PathRuleModule::new(dev("dev.pip"), |ctx| {
        let l = &ctx.env.local_app_data;
        vec![Target::contents(
            "cache",
            [env_or(ctx, "PIP_CACHE_DIR", l.join(r"pip\Cache"))],
        )]
    }));
    e.register(PathRuleModule::new(dev("dev.uv"), |ctx| {
        let def = ctx.env.local_app_data.join(r"uv\cache");
        vec![Target::contents(
            "cache",
            [env_or(ctx, "UV_CACHE_DIR", def)],
        )]
    }));
    e.register(PathRuleModule::new(dev("dev.poetry"), |ctx| {
        let def = ctx.env.local_app_data.join(r"pypoetry\Cache");
        vec![Target::contents(
            "cache",
            [env_or(ctx, "POETRY_CACHE_DIR", def)],
        )]
    }));
    e.register(PathRuleModule::new(dev("dev.conda"), |ctx| {
        let h = &ctx.env.home;
        let mut roots: Vec<PathBuf> = [
            "anaconda3",
            "miniconda3",
            "miniforge3",
            "mambaforge",
            ".conda",
        ]
        .iter()
        .map(|d| h.join(d).join("pkgs"))
        .collect();
        if let Some(dirs) = ctx.env.var("CONDA_PKGS_DIRS") {
            roots.extend(std::env::split_paths(dirs));
        }
        vec![
            Target::contents("tarballs", roots)
                .names(&["*.conda", "*.tar.bz2"])
                .max_depth(1),
        ]
    }));

    // ── JavaScript ──────────────────────────────────────────────────────────
    e.register(PathRuleModule::new(dev("dev.npm"), |ctx| {
        let def = ctx.env.local_app_data.join("npm-cache");
        vec![Target::contents(
            "cache",
            [
                env_or(ctx, "NPM_CONFIG_CACHE", def),
                ctx.env.roaming_app_data.join("npm-cache"),
            ],
        )]
    }));
    e.register(PathRuleModule::new(
        dev("dev.pnpm").risk(Risk::Moderate),
        |ctx| {
            let l = &ctx.env.local_app_data;
            vec![Target::contents(
                "store",
                [l.join(r"pnpm\store"), l.join("pnpm-store")],
            )]
        },
    ));
    e.register(PathRuleModule::new(dev("dev.yarn"), |ctx| {
        let def = ctx.env.local_app_data.join(r"Yarn\Cache");
        vec![Target::contents(
            "cache",
            [
                env_or(ctx, "YARN_CACHE_FOLDER", def),
                ctx.env.home.join(r".yarn\berry\cache"),
            ],
        )]
    }));
    e.register(PathRuleModule::new(dev("dev.bun"), |ctx| {
        let def = ctx.env.home.join(r".bun\install\cache");
        vec![Target::contents(
            "cache",
            [env_or(ctx, "BUN_INSTALL_CACHE_DIR", def)],
        )]
    }));

    // ── Rust / Go / JVM / .NET ──────────────────────────────────────────────
    e.register(PathRuleModule::new(
        dev("dev.cargo").risk(Risk::Moderate),
        |ctx| {
            let c = env_or(ctx, "CARGO_HOME", ctx.env.home.join(".cargo"));
            vec![Target::contents(
                "registry",
                [
                    c.join(r"registry\cache"),
                    c.join(r"registry\src"),
                    c.join(r"git\checkouts"),
                ],
            )]
        },
    ));
    e.register(PathRuleModule::new(dev("dev.go"), |ctx| {
        let gopath = env_or(ctx, "GOPATH", ctx.env.home.join("go"));
        vec![
            Target::contents(
                "build",
                [env_or(
                    ctx,
                    "GOCACHE",
                    ctx.env.local_app_data.join("go-build"),
                )],
            ),
            Target::contents("mod", [env_or(ctx, "GOMODCACHE", gopath.join(r"pkg\mod"))])
                .risk(Risk::Moderate),
        ]
    }));
    e.register(PathRuleModule::new(
        dev("dev.gradle").risk(Risk::Moderate),
        |ctx| {
            let g = env_or(ctx, "GRADLE_USER_HOME", ctx.env.home.join(".gradle"));
            vec![
                Target::contents("caches", [g.join("caches")]),
                Target::contents("wrapper", [g.join(r"wrapper\dists")]),
                Target::contents("daemon", [g.join("daemon")])
                    .names(&["*.log", "*.out.log"])
                    .risk(Risk::Safe),
            ]
        },
    ));
    e.register(PathRuleModule::new(
        dev("dev.maven").risk(Risk::Moderate),
        |ctx| {
            vec![Target::contents(
                "repo",
                [ctx.env.home.join(r".m2\repository")],
            )]
        },
    ));
    e.register(PathRuleModule::new(dev("dev.nuget"), |ctx| {
        let l = &ctx.env.local_app_data;
        vec![
            Target::contents(
                "http",
                [l.join(r"NuGet\v3-cache"), l.join(r"NuGet\plugins-cache")],
            ),
            Target::contents(
                "packages",
                [env_or(
                    ctx,
                    "NUGET_PACKAGES",
                    ctx.env.home.join(r".nuget\packages"),
                )],
            )
            .risk(Risk::Moderate),
        ]
    }));

    // ── Editores ────────────────────────────────────────────────────────────
    e.register(PathRuleModule::new(
        dev("dev.editors").closes(&["code.exe", "cursor.exe", "vscodium.exe", "windsurf.exe"]),
        |ctx| {
            const EDITORS: [(&str, &str); 4] = [
                ("Code", "VS Code"),
                ("Cursor", "Cursor"),
                ("VSCodium", "VSCodium"),
                ("Windsurf", "Windsurf"),
            ];
            const SUB: [&str; 7] = [
                "Cache",
                "CachedData",
                "CachedExtensionVSIXs",
                "Code Cache",
                "GPUCache",
                "logs",
                r"Crashpad\completed",
            ];
            EDITORS
                .iter()
                .map(|(dir, name)| {
                    let base = ctx.env.roaming_app_data.join(dir);
                    Target::contents(dir, SUB.map(|s| base.join(s)))
                        .label(tf("module.dev.editors.cache_of", &[("name", name)]))
                })
                .collect()
        },
    ));
    e.register(PathRuleModule::new(
        dev("dev.jetbrains").risk(Risk::Moderate).closes(&[
            "idea64.exe",
            "pycharm64.exe",
            "webstorm64.exe",
            "rider64.exe",
            "clion64.exe",
            "goland64.exe",
            "rustrover64.exe",
            "phpstorm64.exe",
            "datagrip64.exe",
        ]),
        |ctx| {
            let base = ctx.env.local_app_data.join("JetBrains");
            let mut roots = glob_under(&base, r"*\caches");
            roots.extend(glob_under(&base, r"*\log"));
            vec![Target::contents("caches", roots)]
        },
    ));
}
