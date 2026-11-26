//! Cachés globales de gestores de paquetes y editores.
//!
//! Cada caché respeta su variable de entorno (`PIP_CACHE_DIR`, `CARGO_HOME`…)
//! y si no existe usa la ruta por defecto en Windows.

use crp_core::rules::glob_under;
use crp_core::{Category, Engine, ModuleMeta, PathRuleModule, Risk, ScanContext, Target};
use std::path::PathBuf;

/// `$VAR` si está definida, si no `default`.
fn env_or(ctx: &ScanContext, var: &str, default: PathBuf) -> PathBuf {
    ctx.env.var_path(var).unwrap_or(default)
}

fn dev(id: &str, name: &str, description: &str) -> ModuleMeta {
    ModuleMeta::new(id, name, Category::Dev).describe(description)
}

pub fn register(e: &mut Engine) {
    // ── Python ──────────────────────────────────────────────────────────────
    e.register(PathRuleModule::new(
        dev(
            "dev.pip",
            "Caché de pip",
            "Paquetes descargados por pip (wheels y http). Equivale a `pip cache purge`.",
        ),
        |ctx| {
            let l = &ctx.env.local_app_data;
            vec![Target::contents(
                "cache",
                "Caché de pip",
                [env_or(ctx, "PIP_CACHE_DIR", l.join(r"pip\Cache"))],
            )]
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.uv",
            "Caché de uv",
            "Caché de uv (Astral). Los entornos ya creados siguen funcionando.",
        ),
        |ctx| {
            let def = ctx.env.local_app_data.join(r"uv\cache");
            vec![Target::contents(
                "cache",
                "Caché de uv",
                [env_or(ctx, "UV_CACHE_DIR", def)],
            )]
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.poetry",
            "Caché de Poetry",
            "Paquetes y artefactos descargados por Poetry.",
        ),
        |ctx| {
            let def = ctx.env.local_app_data.join(r"pypoetry\Cache");
            vec![Target::contents(
                "cache",
                "Caché de Poetry",
                [env_or(ctx, "POETRY_CACHE_DIR", def)],
            )]
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.conda",
            "Paquetes de conda",
            "Tarballs (.conda/.tar.bz2) ya extraídos en pkgs. Equivale a `conda clean --tarballs`.",
        ),
        |ctx| {
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
                Target::contents("tarballs", "Tarballs de conda", roots)
                    .names(&["*.conda", "*.tar.bz2"])
                    .max_depth(1),
            ]
        },
    ));

    // ── JavaScript ──────────────────────────────────────────────────────────
    e.register(PathRuleModule::new(
        dev(
            "dev.npm",
            "Caché de npm",
            "Caché de npm (_cacache, _npx, _logs). Equivale a `npm cache clean --force`.",
        ),
        |ctx| {
            let def = ctx.env.local_app_data.join("npm-cache");
            vec![Target::contents(
                "cache",
                "Caché de npm",
                [
                    env_or(ctx, "NPM_CONFIG_CACHE", def),
                    ctx.env.roaming_app_data.join("npm-cache"),
                ],
            )]
        },
    ));
    e.register(PathRuleModule::new(
        dev("dev.pnpm", "Almacén de pnpm", "Content-addressable store de pnpm. Los node_modules existentes siguen funcionando (hardlinks).")
            .risk(Risk::Moderate),
        |ctx| {
            let l = &ctx.env.local_app_data;
            vec![Target::contents("store", "Store de pnpm", [l.join(r"pnpm\store"), l.join("pnpm-store")])
                .detail("Tras limpiar, pnpm volverá a descargar paquetes al instalar.")]
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.yarn",
            "Caché de Yarn",
            "Caché de Yarn clásico y Yarn Berry.",
        ),
        |ctx| {
            let def = ctx.env.local_app_data.join(r"Yarn\Cache");
            vec![Target::contents(
                "cache",
                "Caché de Yarn",
                [
                    env_or(ctx, "YARN_CACHE_FOLDER", def),
                    ctx.env.home.join(r".yarn\berry\cache"),
                ],
            )]
        },
    ));
    e.register(PathRuleModule::new(
        dev("dev.bun", "Caché de Bun", "Paquetes descargados por Bun."),
        |ctx| {
            let def = ctx.env.home.join(r".bun\install\cache");
            vec![Target::contents(
                "cache",
                "Caché de Bun",
                [env_or(ctx, "BUN_INSTALL_CACHE_DIR", def)],
            )]
        },
    ));

    // ── Rust / Go / JVM / .NET ──────────────────────────────────────────────
    e.register(PathRuleModule::new(
        dev("dev.cargo", "Caché de Cargo", "Crates descargados y extraídos (registry/cache, registry/src, git/checkouts).")
            .risk(Risk::Moderate),
        |ctx| {
            let c = env_or(ctx, "CARGO_HOME", ctx.env.home.join(".cargo"));
            vec![Target::contents(
                "registry",
                "Caché de Cargo",
                [c.join(r"registry\cache"), c.join(r"registry\src"), c.join(r"git\checkouts")],
            )
            .detail("Se vuelven a descargar en la próxima compilación (no funcionará sin conexión).")]
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.go",
            "Cachés de Go",
            "Caché de compilación (go-build) y módulos descargados.",
        ),
        |ctx| {
            let gopath = env_or(ctx, "GOPATH", ctx.env.home.join("go"));
            vec![
                Target::contents(
                    "build",
                    "Caché de compilación de Go",
                    [env_or(
                        ctx,
                        "GOCACHE",
                        ctx.env.local_app_data.join("go-build"),
                    )],
                ),
                Target::contents(
                    "mod",
                    "Módulos de Go",
                    [env_or(ctx, "GOMODCACHE", gopath.join(r"pkg\mod"))],
                )
                .risk(Risk::Moderate),
            ]
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.gradle",
            "Cachés de Gradle",
            "Dependencias, distribuciones del wrapper y logs del daemon.",
        )
        .risk(Risk::Moderate),
        |ctx| {
            let g = env_or(ctx, "GRADLE_USER_HOME", ctx.env.home.join(".gradle"));
            vec![
                Target::contents("caches", "Caché de Gradle", [g.join("caches")]),
                Target::contents(
                    "wrapper",
                    "Distribuciones del Gradle Wrapper",
                    [g.join(r"wrapper\dists")],
                ),
                Target::contents("daemon", "Logs del daemon de Gradle", [g.join("daemon")])
                    .names(&["*.log", "*.out.log"])
                    .risk(Risk::Safe),
            ]
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.maven",
            "Repositorio de Maven",
            "Dependencias descargadas en ~/.m2/repository.",
        )
        .risk(Risk::Moderate),
        |ctx| {
            vec![Target::contents(
                "repo",
                "Repositorio local de Maven",
                [ctx.env.home.join(r".m2\repository")],
            )]
        },
    ));
    e.register(PathRuleModule::new(
        dev("dev.nuget", "Cachés de NuGet", "Paquetes globales y cachés HTTP de NuGet. Equivale a `dotnet nuget locals all --clear`."),
        |ctx| {
            let l = &ctx.env.local_app_data;
            vec![
                Target::contents("http", "Caché HTTP de NuGet", [l.join(r"NuGet\v3-cache"), l.join(r"NuGet\plugins-cache")]),
                Target::contents("packages", "Paquetes globales de NuGet",
                    [env_or(ctx, "NUGET_PACKAGES", ctx.env.home.join(r".nuget\packages"))])
                    .risk(Risk::Moderate),
            ]
        },
    ));

    // ── Editores ────────────────────────────────────────────────────────────
    e.register(PathRuleModule::new(
        dev("dev.editors", "Cachés de editores", "VS Code, Cursor, VSCodium y Windsurf: cachés, VSIX descargados y logs. No toca extensiones ni ajustes.")
            .closes(&["code.exe", "cursor.exe", "vscodium.exe", "windsurf.exe"]),
        |ctx| {
            const EDITORS: [(&str, &str); 4] =
                [("Code", "VS Code"), ("Cursor", "Cursor"), ("VSCodium", "VSCodium"), ("Windsurf", "Windsurf")];
            const SUB: [&str; 7] =
                ["Cache", "CachedData", "CachedExtensionVSIXs", "Code Cache", "GPUCache", "logs", r"Crashpad\completed"];
            EDITORS
                .iter()
                .map(|(dir, name)| {
                    let base = ctx.env.roaming_app_data.join(dir);
                    Target::contents(dir, &format!("Caché de {name}"), SUB.map(|s| base.join(s)))
                })
                .collect()
        },
    ));
    e.register(PathRuleModule::new(
        dev(
            "dev.jetbrains",
            "Cachés de JetBrains",
            "Cachés e índices de IntelliJ, PyCharm, WebStorm, Rider… (se re-indexa al abrir).",
        )
        .risk(Risk::Moderate)
        .closes(&[
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
            vec![Target::contents(
                "caches",
                "Cachés de IDEs JetBrains",
                roots,
            )]
        },
    ));
}
