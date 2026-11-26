//! Limpieza del sistema Windows.
//!
//! No incluye Prefetch a propósito: borrarlo empeora el arranque.

use crp_core::{
    Action, Category, CleanOutcome, CleanerModule, Engine, Error, Finding, ModuleMeta,
    PathRuleModule, Result, Risk, ScanContext, Target,
};
use crp_platform::recycle_bin;

pub fn register(e: &mut Engine) {
    e.register(PathRuleModule::new(
        ModuleMeta::new("system.user_temp", "Temporales del usuario", Category::System)
            .describe("Archivos de %TEMP% con más de 24 h. Los recientes se conservan por si hay una instalación en curso."),
        |ctx| {
            vec![Target::contents("temp", "Temporales del usuario (%TEMP%)", [ctx.env.temp.clone()])
                .older_than_hours(24)]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new(
            "system.windows_temp",
            "Temporales de Windows",
            Category::System,
        )
        .describe("C:\\Windows\\Temp con más de 24 h.")
        .admin(),
        |ctx| {
            vec![
                Target::contents(
                    "temp",
                    "Temporales de Windows",
                    [ctx.env.windows_dir.join("Temp")],
                )
                .older_than_hours(24),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.thumbnails", "Miniaturas", Category::System)
            .describe("Caché de miniaturas del Explorador (thumbcache_*.db). Se regenera al navegar carpetas."),
        |ctx| {
            vec![Target::contents(
                "thumbcache",
                "Caché de miniaturas",
                [ctx.env.local_app_data.join(r"Microsoft\Windows\Explorer")],
            )
            .names(&["thumbcache_*.db"])
            .max_depth(1)
            .detail("El Explorador bloquea algunas; esas se omiten.")]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.wer", "Informes de errores", Category::System).describe(
            "Informes de Windows Error Reporting (bloqueos de apps) ya enviados o en cola.",
        ),
        |ctx| {
            const SUB: [&str; 3] = ["ReportArchive", "ReportQueue", "Temp"];
            let user = ctx.env.local_app_data.join(r"Microsoft\Windows\WER");
            let system = ctx.env.program_data.join(r"Microsoft\Windows\WER");
            vec![
                Target::contents(
                    "user",
                    "Informes de errores (usuario)",
                    SUB.map(|s| user.join(s)),
                ),
                Target::contents(
                    "system",
                    "Informes de errores (sistema)",
                    SUB.map(|s| system.join(s)),
                )
                .admin(true),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new(
            "system.crash_dumps",
            "Volcados de memoria",
            Category::System,
        )
        .describe(
            "Volcados generados cuando una app o Windows se cuelga. Sólo útiles para depurar.",
        ),
        |ctx| {
            let w = &ctx.env.windows_dir;
            vec![
                Target::contents(
                    "user",
                    "Volcados de aplicaciones",
                    [ctx.env.local_app_data.join("CrashDumps")],
                ),
                Target::contents(
                    "minidump",
                    "Minidumps del sistema",
                    [w.join("Minidump"), w.join("LiveKernelReports")],
                )
                .admin(true),
                Target::paths("memory", "MEMORY.DMP", [w.join("MEMORY.DMP")]).admin(true),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new(
            "system.windows_logs",
            "Registros de Windows",
            Category::System,
        )
        .describe("Archivos de C:\\Windows\\Logs con más de 7 días (CBS, DISM, WindowsUpdate…).")
        .risk(Risk::Moderate)
        .admin(),
        |ctx| {
            vec![
                Target::contents(
                    "logs",
                    "Logs de Windows (+7 días)",
                    [ctx.env.windows_dir.join("Logs")],
                )
                .older_than_hours(7 * 24),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.shader_cache", "Cachés de shaders", Category::System)
            .describe("Shaders compilados de DirectX y de los drivers NVIDIA/AMD/Intel. Se recompilan al jugar (puede haber tirones la primera vez)."),
        |ctx| {
            let l = &ctx.env.local_app_data;
            let ll = ctx.env.local_low();
            vec![
                Target::contents("d3d", "DirectX (D3DSCache)", [l.join("D3DSCache")]),
                Target::contents(
                    "gpu",
                    "Drivers de GPU",
                    [
                        l.join(r"NVIDIA\DXCache"),
                        l.join(r"NVIDIA\GLCache"),
                        l.join(r"NVIDIA Corporation\NV_Cache"),
                        ll.join(r"NVIDIA\PerDriverVersion\DXCache"),
                        l.join(r"AMD\DxCache"),
                        l.join(r"AMD\DxcCache"),
                        l.join(r"AMD\VkCache"),
                        l.join(r"AMD\GLCache"),
                        ll.join(r"Intel\ShaderCache"),
                    ],
                ),
                Target::contents(
                    "steam",
                    "Steam (shadercache)",
                    [ctx.env.program_files_x86.join(r"Steam\steamapps\shadercache")],
                )
                .risk(Risk::Moderate)
                .detail("Steam los vuelve a descargar o compilar al abrir cada juego."),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.delivery_optimization", "Delivery Optimization", Category::System)
            .describe("Caché P2P de actualizaciones de Windows y de la Store.")
            .risk(Risk::Moderate)
            .admin(),
        |ctx| {
            vec![Target::contents(
                "cache",
                "Caché de Delivery Optimization",
                [ctx.env.windows_dir.join(
                    r"ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache",
                )],
            )]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new(
            "system.windows_update",
            "Descargas de Windows Update",
            Category::System,
        )
        .describe("Paquetes ya descargados en SoftwareDistribution\\Download.")
        .risk(Risk::Moderate)
        .admin(),
        |ctx| {
            vec![
                Target::contents(
                    "download",
                    "Descargas de Windows Update",
                    [ctx.env.windows_dir.join(r"SoftwareDistribution\Download")],
                )
                .detail(
                    "Si hay una actualización en curso, sus archivos estarán en uso y se omitirán.",
                ),
            ]
        },
    ));

    e.register(RecycleBin {
        meta: ModuleMeta::new("system.recycle_bin", "Papelera de reciclaje", Category::System)
            .describe("Vacía la papelera de todas las unidades. Lo que haya dentro se elimina definitivamente.")
            .risk(Risk::Moderate),
    });
}

/// Papelera vía Win32 (`SHQueryRecycleBin` / `SHEmptyRecycleBin`).
struct RecycleBin {
    meta: ModuleMeta,
}

impl CleanerModule for RecycleBin {
    fn meta(&self) -> &ModuleMeta {
        &self.meta
    }

    fn scan(&self, _ctx: &ScanContext) -> Result<Vec<Finding>> {
        let info = recycle_bin::query().map_err(|e| Error::Other(e.to_string()))?;
        if info.items == 0 {
            return Ok(Vec::new());
        }
        Ok(vec![Finding {
            id: format!("{}:all", self.meta.id),
            module_id: self.meta.id.clone(),
            category: self.meta.category,
            label: "Papelera de reciclaje".into(),
            detail: Some("Se elimina definitivamente.".into()),
            path: None,
            bytes: info.bytes,
            items: info.items,
            risk: self.meta.risk,
            requires_admin: false,
            selected: false,
            age_days: None,
            warning: None,
            action: Action::Custom {
                key: "empty".into(),
            },
        }])
    }

    fn clean_custom(&self, _finding: &Finding, _ctx: &ScanContext) -> Result<CleanOutcome> {
        let before = recycle_bin::query().map_err(|e| Error::Other(e.to_string()))?;
        recycle_bin::empty().map_err(|e| Error::Other(e.to_string()))?;
        Ok(CleanOutcome {
            freed_bytes: before.bytes,
            removed_items: before.items,
            ..Default::default()
        })
    }
}
