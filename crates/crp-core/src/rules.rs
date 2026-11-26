//! Módulos **declarativos**: un módulo describe *qué rutas* limpiar y el
//! motor se encarga de medir y borrar. Así añadir un limpiador nuevo son
//! unas pocas líneas sin lógica de E/S.
//!
//! ```ignore
//! PathRuleModule::new(
//!     ModuleMeta::new("dev.pip", Category::Dev),
//!     |ctx| vec![Target::contents("cache", [ctx.env.local_app_data.join(r"pip\Cache")])],
//! )
//! ```
//!
//! Los textos van en `locales/*.json`: `module.dev.pip.name`, `.desc`,
//! `.target.cache` (etiqueta) y opcionalmente `.detail.cache`.

use crate::context::{ScanContext, path_within};
use crate::error::{Error, Result};
use crate::fsutil::{self, Stats};
use crate::i18n;
use crate::model::{Action, EntryFilter, Finding, Risk};
use crate::module::{CleanerModule, ModuleMeta};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum TargetKind {
    /// Vaciar carpetas (conservándolas) con un filtro.
    Contents(EntryFilter),
    /// Borrar rutas completas.
    Paths,
}

/// Un objetivo de limpieza declarado por un módulo.
#[derive(Debug, Clone)]
pub struct Target {
    pub key: String,
    /// `None` = `module.<id>.target.<key>` de los locales.
    pub label: Option<String>,
    pub kind: TargetKind,
    pub roots: Vec<PathBuf>,
    pub risk: Option<Risk>,
    pub requires_admin: Option<bool>,
    pub detail: Option<String>,
    pub age_days: Option<u64>,
}

impl Target {
    fn new(key: &str, kind: TargetKind, roots: Vec<PathBuf>) -> Self {
        Self {
            key: key.to_owned(),
            label: None,
            kind,
            roots,
            risk: None,
            requires_admin: None,
            detail: None,
            age_days: None,
        }
    }

    /// Vaciar el contenido de estas carpetas.
    pub fn contents(key: &str, roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self::new(
            key,
            TargetKind::Contents(EntryFilter::default()),
            roots.into_iter().collect(),
        )
    }

    /// Borrar estas rutas completas.
    pub fn paths(key: &str, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        Self::new(key, TargetKind::Paths, paths.into_iter().collect())
    }

    fn filter_mut(&mut self) -> Option<&mut EntryFilter> {
        match &mut self.kind {
            TargetKind::Contents(f) => Some(f),
            TargetKind::Paths => None,
        }
    }

    /// Sólo archivos cuyo nombre case con alguno de estos globs.
    pub fn names(mut self, patterns: &[&str]) -> Self {
        if let Some(f) = self.filter_mut() {
            f.names = patterns.iter().map(|p| (*p).to_owned()).collect();
        }
        self
    }

    /// Sólo archivos modificados hace al menos `hours` horas.
    pub fn older_than_hours(mut self, hours: u64) -> Self {
        if let Some(f) = self.filter_mut() {
            f.min_age_secs = Some(hours * 3600);
        }
        self
    }

    pub fn max_depth(mut self, depth: usize) -> Self {
        if let Some(f) = self.filter_mut() {
            f.max_depth = Some(depth);
        }
        self
    }

    pub fn risk(mut self, risk: Risk) -> Self {
        self.risk = Some(risk);
        self
    }

    pub fn admin(mut self, required: bool) -> Self {
        self.requires_admin = Some(required);
        self
    }

    /// Etiqueta propia (nombres de modelos, proyectos…) en vez de la de los locales.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Días desde la última modificación (se muestra y permite ordenar).
    pub fn age_days(mut self, days: Option<u64>) -> Self {
        self.age_days = days;
        self
    }

    /// Mide el objetivo y lo convierte en hallazgo (o `None` si no hay nada).
    ///
    /// Respeta las exclusiones del usuario: se descartan raíces excluidas, las
    /// rutas completas que contengan una exclusión, y las exclusiones dentro de
    /// una carpeta a vaciar se añaden a su filtro.
    pub fn into_finding(mut self, meta: &ModuleMeta, ctx: &ScanContext) -> Option<Finding> {
        let ex = &ctx.exclusions;
        let roots: Vec<PathBuf> = existing_unique(self.roots)
            .into_iter()
            .filter(|r| !ctx.is_excluded(r))
            .filter(|r| match self.kind {
                TargetKind::Paths => !ex.iter().any(|e| path_within(e, r)),
                TargetKind::Contents(_) => true,
            })
            .collect();
        if roots.is_empty() {
            return None;
        }
        if let TargetKind::Contents(f) = &mut self.kind {
            f.exclude = ex
                .iter()
                .filter(|e| roots.iter().any(|r| path_within(e, r)))
                .cloned()
                .collect();
        }
        let mut stats = Stats::default();
        for root in &roots {
            stats += match &self.kind {
                TargetKind::Contents(f) => fsutil::measure_contents(root, f, &ctx.cancel),
                TargetKind::Paths => fsutil::measure_path(root, &ctx.cancel),
            };
        }
        if stats.items == 0 {
            return None;
        }
        let risk = self.risk.unwrap_or(meta.risk);
        let action = match self.kind {
            TargetKind::Contents(filter) => Action::DeleteContents {
                roots: roots.clone(),
                filter,
            },
            TargetKind::Paths => Action::DeletePaths {
                paths: roots.clone(),
            },
        };
        Some(Finding {
            id: format!("{}:{}", meta.id, self.key),
            module_id: meta.id.clone(),
            category: meta.category,
            label: self
                .label
                .unwrap_or_else(|| i18n::t(&format!("module.{}.target.{}", meta.id, self.key))),
            detail: self
                .detail
                .or_else(|| i18n::get(&format!("module.{}.detail.{}", meta.id, self.key))),
            path: roots.into_iter().next(),
            bytes: stats.bytes,
            items: stats.items,
            risk,
            requires_admin: self.requires_admin.unwrap_or(meta.requires_admin),
            selected: risk == Risk::Safe,
            age_days: self.age_days,
            warning: None,
            action,
        })
    }
}

/// Quita rutas inexistentes y duplicadas (sin distinguir mayúsculas).
fn existing_unique(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|p| std::fs::symlink_metadata(p).is_ok())
        .filter(|p| seen.insert(p.to_string_lossy().to_lowercase()))
        .collect()
}

/// Expande un glob (`*`, `?`) bajo `base`, escapando los caracteres especiales de `base`.
pub fn glob_under(base: &Path, pattern: &str) -> Vec<PathBuf> {
    let full = format!(
        "{}\\{}",
        glob::Pattern::escape(&base.to_string_lossy()),
        pattern
    );
    match glob::glob(&full) {
        Ok(paths) => paths.filter_map(std::result::Result::ok).collect(),
        Err(_) => Vec::new(),
    }
}

type Resolver = Box<dyn Fn(&ScanContext) -> Vec<Target> + Send + Sync>;

/// Módulo definido por una función que devuelve [`Target`]s.
pub struct PathRuleModule {
    meta: ModuleMeta,
    resolve: Resolver,
}

impl PathRuleModule {
    pub fn new(
        meta: ModuleMeta,
        resolve: impl Fn(&ScanContext) -> Vec<Target> + Send + Sync + 'static,
    ) -> Self {
        Self {
            meta,
            resolve: Box::new(resolve),
        }
    }
}

impl CleanerModule for PathRuleModule {
    fn meta(&self) -> &ModuleMeta {
        &self.meta
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for target in (self.resolve)(ctx) {
            if ctx.cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            findings.extend(target.into_finding(&self.meta, ctx));
        }
        Ok(findings)
    }
}
