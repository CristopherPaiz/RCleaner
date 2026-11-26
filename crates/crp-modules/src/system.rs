//! Limpieza del sistema Windows.
//!
//! No incluye Prefetch a propósito: borrarlo empeora el arranque.

use crp_core::{
    Action, Category, CleanOutcome, CleanerModule, Engine, Error, Finding, ModuleMeta,
    PathRuleModule, Result, Risk, ScanContext, Target, t,
};
use crp_platform::recycle_bin;

pub fn register(e: &mut Engine) {
    e.register(PathRuleModule::new(
        ModuleMeta::new("system.user_temp", Category::System),
        |ctx| vec![Target::contents("temp", [ctx.env.temp.clone()]).older_than_hours(24)],
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.windows_temp", Category::System).admin(),
        |ctx| {
            vec![Target::contents("temp", [ctx.env.windows_dir.join("Temp")]).older_than_hours(24)]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.thumbnails", Category::System),
        |ctx| {
            vec![
                Target::contents(
                    "thumbcache",
                    [ctx.env.local_app_data.join(r"Microsoft\Windows\Explorer")],
                )
                .names(&["thumbcache_*.db"])
                .max_depth(1),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.wer", Category::System),
        |ctx| {
            const SUB: [&str; 3] = ["ReportArchive", "ReportQueue", "Temp"];
            let user = ctx.env.local_app_data.join(r"Microsoft\Windows\WER");
            let system = ctx.env.program_data.join(r"Microsoft\Windows\WER");
            vec![
                Target::contents("user", SUB.map(|s| user.join(s))),
                Target::contents("system", SUB.map(|s| system.join(s))).admin(true),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.crash_dumps", Category::System),
        |ctx| {
            let w = &ctx.env.windows_dir;
            vec![
                Target::contents("user", [ctx.env.local_app_data.join("CrashDumps")]),
                Target::contents(
                    "minidump",
                    [w.join("Minidump"), w.join("LiveKernelReports")],
                )
                .admin(true),
                Target::paths("memory", [w.join("MEMORY.DMP")]).admin(true),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.windows_logs", Category::System)
            .risk(Risk::Moderate)
            .admin(),
        |ctx| {
            vec![
                Target::contents("logs", [ctx.env.windows_dir.join("Logs")])
                    .older_than_hours(7 * 24),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.shader_cache", Category::System),
        |ctx| {
            let l = &ctx.env.local_app_data;
            let ll = ctx.env.local_low();
            vec![
                Target::contents("d3d", [l.join("D3DSCache")]),
                Target::contents(
                    "gpu",
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
                    [ctx.env
                        .program_files_x86
                        .join(r"Steam\steamapps\shadercache")],
                )
                .risk(Risk::Moderate),
            ]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.delivery_optimization", Category::System)
            .risk(Risk::Moderate)
            .admin(),
        |ctx| {
            vec![Target::contents("cache", [ctx.env.windows_dir.join(
                    r"ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache",
                )],
            )]
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("system.windows_update", Category::System)
            .risk(Risk::Moderate)
            .admin(),
        |ctx| {
            vec![Target::contents(
                "download",
                [ctx.env.windows_dir.join(r"SoftwareDistribution\Download")],
            )]
        },
    ));

    e.register(RecycleBin {
        meta: ModuleMeta::new("system.recycle_bin", Category::System).risk(Risk::Moderate),
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
            label: t("module.system.recycle_bin.name"),
            detail: Some(t("module.system.recycle_bin.detail")),
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
