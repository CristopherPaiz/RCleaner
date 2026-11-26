//! Artefactos regenerables dentro de proyectos: `node_modules`, `target`,
//! `__pycache__`, `.venv`, `.next`…
//!
//! Recorre las carpetas de proyectos del contexto (`ScanContext::dev_roots`),
//! no entra en los artefactos encontrados ni en `.git`, y mide en paralelo.

use crp_core::fsutil::measure_path;
use crp_core::{
    Action, Category, CleanerModule, Error, Finding, ModuleMeta, Result, Risk, ScanContext,
};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

const MAX_DEPTH: usize = 10;

/// Carpetas en las que nunca se entra.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vs",
    "AppData",
    "$RECYCLE.BIN",
];

const PY_CACHES: &[&str] = &["__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache"];
const WEB_CACHES: &[&str] = &[
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".parcel-cache",
    ".angular",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    NodeModules,
    CargoTarget,
    PyCache,
    Venv,
    WebCache,
    GradleProject,
}

impl Kind {
    fn risk(self) -> Risk {
        match self {
            Self::PyCache => Risk::Safe,
            Self::NodeModules | Self::CargoTarget | Self::WebCache | Self::GradleProject => {
                Risk::Moderate
            }
            Self::Venv => Risk::Advanced,
        }
    }

    /// Manifiesto que indica la última actividad del proyecto.
    fn manifest(self) -> &'static [&'static str] {
        match self {
            Self::NodeModules | Self::WebCache => &["package.json"],
            Self::CargoTarget => &["Cargo.toml"],
            Self::GradleProject => &[
                "build.gradle",
                "build.gradle.kts",
                "settings.gradle",
                "settings.gradle.kts",
            ],
            Self::Venv => &["pyproject.toml", "requirements.txt", "setup.py"],
            Self::PyCache => &[],
        }
    }
}

fn has_any(dir: &Path, files: &[&str]) -> bool {
    files.iter().any(|f| dir.join(f).is_file())
}

/// ¿Es esta carpeta un artefacto conocido?
fn classify(path: &Path) -> Option<Kind> {
    let name = path.file_name()?.to_str()?;
    let parent = path.parent()?;
    match name {
        "node_modules" if parent.join("package.json").is_file() => Some(Kind::NodeModules),
        "target" if parent.join("Cargo.toml").is_file() => Some(Kind::CargoTarget),
        ".gradle" if has_any(parent, Kind::GradleProject.manifest()) => Some(Kind::GradleProject),
        n if PY_CACHES.contains(&n) => Some(Kind::PyCache),
        n if WEB_CACHES.contains(&n) && parent.join("package.json").is_file() => {
            Some(Kind::WebCache)
        }
        ".venv" | "venv" | "env" if path.join("pyvenv.cfg").is_file() => Some(Kind::Venv),
        _ => None,
    }
}

/// Busca artefactos bajo `root` sin entrar en ellos.
fn find_artifacts(root: &Path, ctx: &ScanContext) -> Vec<(PathBuf, Kind)> {
    let mut found = Vec::new();
    let mut it = WalkDir::new(root)
        .follow_links(false)
        .max_depth(MAX_DEPTH)
        .into_iter();
    while let Some(entry) = it.next() {
        if ctx.cancel.is_cancelled() {
            break;
        }
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();
        if ctx.is_excluded(path) {
            it.skip_current_dir();
            continue;
        }
        if let Some(kind) = classify(path) {
            found.push((path.to_path_buf(), kind));
            it.skip_current_dir();
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        // node_modules sin package.json al lado: no es nuestro, pero tampoco se recorre.
        if SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(&name)) || name == "node_modules" {
            it.skip_current_dir();
        }
    }
    found
}

/// Días desde la última modificación del manifiesto del proyecto (o de la carpeta).
fn idle_days(project: &Path, kind: Kind) -> Option<u64> {
    let candidates = kind
        .manifest()
        .iter()
        .map(|m| project.join(m))
        .chain([project.to_path_buf()]);
    let newest = candidates
        .filter_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()?;
    SystemTime::now()
        .duration_since(newest)
        .ok()
        .map(|d| d.as_secs() / 86_400)
}

pub struct Projects {
    meta: ModuleMeta,
}

impl Projects {
    pub fn new() -> Self {
        Self {
            meta: ModuleMeta::new("dev.projects", "Artefactos de proyectos", Category::Dev)
                .describe("node_modules, target de Cargo, __pycache__, .venv y cachés de frameworks web dentro de tus carpetas de proyectos. Se regeneran con npm install / cargo build / etc.")
                .risk(Risk::Moderate),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn finding(
        &self,
        key: String,
        label: String,
        detail: String,
        paths: Vec<PathBuf>,
        bytes: u64,
        items: u64,
        risk: Risk,
        age_days: Option<u64>,
    ) -> Finding {
        Finding {
            id: format!("{}:{key}", self.meta.id),
            module_id: self.meta.id.clone(),
            category: self.meta.category,
            label,
            detail: Some(detail),
            path: paths.first().cloned(),
            bytes,
            items,
            risk,
            requires_admin: false,
            selected: risk == Risk::Safe,
            age_days,
            warning: None,
            action: Action::DeletePaths { paths },
        }
    }
}

impl Default for Projects {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanerModule for Projects {
    fn meta(&self) -> &ModuleMeta {
        &self.meta
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<Finding>> {
        let artifacts: Vec<(PathBuf, Kind)> = ctx
            .dev_roots
            .par_iter()
            .flat_map(|r| find_artifacts(r, ctx))
            .collect();
        if ctx.cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }

        let measured: Vec<_> = artifacts
            .into_par_iter()
            .map(|(path, kind)| {
                let stats = measure_path(&path, &ctx.cancel);
                (path, kind, stats)
            })
            .filter(|(_, _, s)| s.items > 0)
            .collect();

        let mut findings = Vec::new();
        let (py, rest): (Vec<_>, Vec<_>) = measured
            .into_iter()
            .partition(|(_, k, _)| *k == Kind::PyCache);

        if !py.is_empty() {
            let bytes = py.iter().map(|(_, _, s)| s.bytes).sum();
            let items = py.iter().map(|(_, _, s)| s.items).sum();
            let n = py.len();
            let paths = py.into_iter().map(|(p, _, _)| p).collect();
            findings.push(self.finding(
                "python-caches".into(),
                "Cachés de Python (__pycache__, .pytest_cache, .mypy_cache, .ruff_cache)".into(),
                format!("{n} carpetas"),
                paths,
                bytes,
                items,
                Risk::Safe,
                None,
            ));
        }

        for (path, kind, stats) in rest {
            let project = path.parent().unwrap_or(&path).to_path_buf();
            let project_name = project.file_name().map_or_else(
                || project.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let days = idle_days(&project, kind);
            let idle = days.map_or_else(String::new, |d| format!(" · sin cambios hace {d} días"));
            findings.push(self.finding(
                path.to_string_lossy().to_lowercase(),
                format!("{dir_name} · {project_name}"),
                format!("{}{idle}", project.display()),
                vec![path],
                stats.bytes,
                stats.items,
                kind.risk(),
                days,
            ));
        }
        findings.sort_by_key(|f| std::cmp::Reverse(f.bytes));
        Ok(findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crp_core::Env;
    use std::fs;

    #[test]
    fn finds_artifacts_and_skips_inside_them() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("web");
        fs::create_dir_all(proj.join(r"node_modules\dep\node_modules\x")).unwrap();
        fs::write(proj.join("package.json"), "{}").unwrap();
        fs::write(proj.join(r"node_modules\dep\index.js"), "x").unwrap();
        fs::create_dir_all(proj.join(r"src\__pycache__")).unwrap();
        fs::write(proj.join(r"src\__pycache__\a.pyc"), "pyc").unwrap();
        // "target" sin Cargo.toml no es un artefacto.
        fs::create_dir_all(proj.join("target")).unwrap();
        fs::write(proj.join(r"target\keep.txt"), "k").unwrap();

        let ctx = ScanContext::new(Env::sandbox(dir.path()), false)
            .with_dev_roots(vec![dir.path().to_path_buf()]);
        let findings = Projects::new().scan(&ctx).unwrap();

        assert_eq!(findings.len(), 2, "{findings:#?}");
        let nm = findings
            .iter()
            .find(|f| f.label.starts_with("node_modules"))
            .unwrap();
        assert_eq!(nm.risk, Risk::Moderate);
        assert!(!nm.selected);
        let py = findings
            .iter()
            .find(|f| f.id.ends_with("python-caches"))
            .unwrap();
        assert!(py.selected);
    }
}
