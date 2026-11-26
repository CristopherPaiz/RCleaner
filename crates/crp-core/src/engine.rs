//! Motor: registro de módulos, escaneo paralelo y limpieza.

use crate::context::ScanContext;
use crate::error::Error;
use crate::exec;
use crate::model::{CleanOutcome, CleanReport, Finding, ModuleError, ScanReport};
use crate::modes;
use crate::module::{CleanerModule, ModuleMeta};
use crate::progress::{ProgressEvent, ProgressSink};
use rayon::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Debug, Clone, Copy, Default)]
pub struct CleanOptions {
    /// No borra nada; sólo informa lo que se liberaría.
    pub dry_run: bool,
}

#[derive(Default, Clone)]
pub struct Engine {
    modules: Vec<Arc<dyn CleanerModule>>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra un módulo. Panica si el id ya existe (error de programación).
    pub fn register(&mut self, module: impl CleanerModule + 'static) -> &mut Self {
        let id = &module.meta().id;
        assert!(self.module(id).is_none(), "módulo duplicado: {id}");
        self.modules.push(Arc::new(module));
        self
    }

    pub fn metas(&self) -> impl Iterator<Item = &ModuleMeta> {
        self.modules.iter().map(|m| m.meta())
    }

    pub fn module(&self, id: &str) -> Option<&Arc<dyn CleanerModule>> {
        self.modules.iter().find(|m| m.meta().id == id)
    }

    /// Módulos que cumplen los selectores (ver [`crate::modes`]).
    pub fn resolve(&self, selectors: &[String]) -> Vec<Arc<dyn CleanerModule>> {
        self.modules
            .iter()
            .filter(|m| modes::selected(selectors, &m.meta().id))
            .cloned()
            .collect()
    }

    /// Escanea en paralelo. Nunca modifica el disco.
    pub fn scan(
        &self,
        selectors: &[String],
        ctx: &ScanContext,
        sink: &dyn ProgressSink,
    ) -> ScanReport {
        let start = Instant::now();
        let modules = self.resolve(selectors);
        let total = modules.len();
        sink.emit(ProgressEvent::ScanStarted { total });
        let done = AtomicUsize::new(0);

        let results: Vec<_> = modules
            .par_iter()
            .map(|m| {
                let id = m.meta().id.clone();
                sink.emit(ProgressEvent::ModuleStarted {
                    module_id: id.clone(),
                });
                let result = if ctx.cancel.is_cancelled() {
                    Err(Error::Cancelled.to_string())
                } else {
                    // Un módulo con un bug no debe tumbar todo el escaneo.
                    match catch_unwind(AssertUnwindSafe(|| m.scan(ctx))) {
                        Ok(r) => r.map_err(|e| e.to_string()),
                        Err(_) => Err(crate::i18n::t("core.module_panic")),
                    }
                };
                let result = result.map(|mut findings| {
                    warn_if_running(m.meta(), ctx, &mut findings);
                    findings
                });
                let (count, bytes) = match &result {
                    Ok(f) => (f.len(), f.iter().map(|x| x.bytes).sum()),
                    Err(_) => (0, 0),
                };
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                sink.emit(ProgressEvent::ModuleFinished {
                    module_id: id.clone(),
                    done: n,
                    total,
                    findings: count,
                    bytes,
                });
                (id, result)
            })
            .collect();

        let mut report = ScanReport::default();
        for (module_id, result) in results {
            match result {
                Ok(findings) => report.findings.extend(findings),
                Err(message) => report.errors.push(ModuleError { module_id, message }),
            }
        }
        report.elapsed_ms = start.elapsed().as_millis();
        report
    }

    /// Limpia los hallazgos indicados (normalmente los que el usuario marcó).
    pub fn clean(
        &self,
        findings: &[Finding],
        opts: CleanOptions,
        ctx: &ScanContext,
        sink: &dyn ProgressSink,
    ) -> CleanReport {
        let start = Instant::now();
        let total = findings.len();
        sink.emit(ProgressEvent::CleanStarted { total });
        let done = AtomicUsize::new(0);

        let outcomes = findings
            .par_iter()
            .map(|f| {
                let outcome = match self.module(&f.module_id) {
                    Some(m) => exec::execute(f, m.as_ref(), ctx, opts.dry_run),
                    None => CleanOutcome {
                        finding_id: f.id.clone(),
                        errors: vec![Error::UnknownModule(f.module_id.clone()).to_string()],
                        ..Default::default()
                    },
                };
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                sink.emit(ProgressEvent::FindingCleaned {
                    finding_id: f.id.clone(),
                    done: n,
                    total,
                    freed_bytes: outcome.freed_bytes,
                });
                outcome
            })
            .collect();

        CleanReport {
            outcomes,
            dry_run: opts.dry_run,
            elapsed_ms: start.elapsed().as_millis(),
        }
    }
}

/// Si algún programa del módulo está abierto, avisa en sus hallazgos.
fn warn_if_running(meta: &ModuleMeta, ctx: &ScanContext, findings: &mut [Finding]) {
    let open: Vec<&str> = meta
        .processes
        .iter()
        .filter(|p| ctx.running.contains(*p))
        .map(String::as_str)
        .collect();
    if open.is_empty() {
        return;
    }
    let msg = crate::i18n::tf(
        "core.app_running",
        &[("name", &meta.name), ("exe", &open.join(", "))],
    );
    for f in findings.iter_mut().filter(|f| f.warning.is_none()) {
        f.warning = Some(msg.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;
    use crate::model::Category;
    use crate::progress::NoProgress;
    use crate::rules::{PathRuleModule, Target};

    #[test]
    fn scan_then_clean_sandbox() {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::sandbox(dir.path());
        let cache = env.local_app_data.join(r"demo\Cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("x.bin"), [0u8; 64]).unwrap();

        let mut engine = Engine::new();
        engine.register(PathRuleModule::new(
            ModuleMeta::new("test.demo", Category::System),
            move |ctx| {
                vec![Target::contents(
                    "c",
                    [ctx.env.local_app_data.join(r"demo\Cache")],
                )]
            },
        ));
        let ctx = ScanContext::new(env, false);
        let sel = vec!["test.*".to_string()];

        let report = engine.scan(&sel, &ctx, &NoProgress);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.total_bytes(), 64);
        assert!(cache.join("x.bin").exists(), "escanear no borra");

        let dry = engine.clean(
            &report.findings,
            CleanOptions { dry_run: true },
            &ctx,
            &NoProgress,
        );
        assert_eq!(dry.freed_bytes(), 64);
        assert!(cache.join("x.bin").exists(), "dry-run no borra");

        let real = engine.clean(&report.findings, CleanOptions::default(), &ctx, &NoProgress);
        assert_eq!(real.freed_bytes(), 64);
        assert!(!cache.join("x.bin").exists());
    }

    #[test]
    fn running_process_adds_warning_and_exclusions_apply() {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::sandbox(dir.path());
        let cache = env.local_app_data.join("b");
        std::fs::create_dir_all(cache.join("keep")).unwrap();
        std::fs::write(cache.join(r"keep\x"), [0u8; 4]).unwrap();
        std::fs::write(cache.join("y"), [0u8; 6]).unwrap();

        let mut engine = Engine::new();
        engine.register(PathRuleModule::new(
            ModuleMeta::new("test.b", Category::Browser).closes(&["Nav.exe"]),
            |ctx| vec![Target::contents("c", [ctx.env.local_app_data.join("b")])],
        ));
        let mut ctx = ScanContext::new(env, false)
            .with_running(["nav.exe".to_string()].into_iter().collect());
        ctx.exclusions = vec![cache.join("keep")];
        let report = engine.scan(&["*".into()], &ctx, &NoProgress);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].bytes, 6, "la exclusión no se mide");
        assert!(
            report.findings[0]
                .warning
                .as_deref()
                .unwrap()
                .contains("nav.exe")
        );

        let r = engine.clean(&report.findings, CleanOptions::default(), &ctx, &NoProgress);
        assert_eq!(r.freed_bytes(), 6);
        assert!(cache.join(r"keep\x").exists());
        assert!(!cache.join("y").exists());
    }

    #[test]
    fn admin_findings_skipped_without_elevation() {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::sandbox(dir.path());
        let t = env.windows_dir.join("Temp");
        std::fs::create_dir_all(&t).unwrap();
        std::fs::write(t.join("a"), [1u8; 8]).unwrap();

        let mut engine = Engine::new();
        engine.register(PathRuleModule::new(
            ModuleMeta::new("test.win", Category::System).admin(),
            |ctx| vec![Target::contents("t", [ctx.env.windows_dir.join("Temp")])],
        ));
        let ctx = ScanContext::new(env, false);
        let report = engine.scan(&["*".into()], &ctx, &NoProgress);
        let res = engine.clean(&report.findings, CleanOptions::default(), &ctx, &NoProgress);
        assert_eq!(res.freed_bytes(), 0);
        assert!(t.join("a").exists());
    }
}
