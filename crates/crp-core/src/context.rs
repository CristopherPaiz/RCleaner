use crate::env::Env;
use crate::settings::Settings;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Mando de una tarea compartido entre la UI y los hilos de trabajo:
/// bandera de cancelación y la ruta que se está tocando ahora mismo (para
/// enseñar qué se elimina).
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
    current: Arc<Mutex<Option<PathBuf>>>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Anota la ruta en curso. Si otro hilo la está anotando, se omite: es
    /// sólo informativa y no debe frenar el borrado.
    pub fn set_current(&self, path: &Path) {
        if let Ok(mut c) = self.current.try_lock() {
            *c = Some(path.to_path_buf());
        }
    }

    /// Última ruta anotada con [`set_current`](Self::set_current).
    pub fn current(&self) -> Option<PathBuf> {
        self.current.lock().ok().and_then(|c| c.clone())
    }
}

/// Todo lo que un módulo necesita para escanear o limpiar.
#[derive(Debug, Clone)]
pub struct ScanContext {
    pub env: Env,
    pub cancel: CancelToken,
    /// El proceso corre elevado.
    pub is_admin: bool,
    /// Carpetas donde buscar proyectos de código (`node_modules`, `target`…).
    pub dev_roots: Vec<PathBuf>,
    /// Rutas que el usuario nunca quiere limpiar.
    pub exclusions: Vec<PathBuf>,
    /// Nombres (en minúsculas) de los ejecutables en marcha.
    pub running: HashSet<String>,
    /// Incluir datos privados de los navegadores (cookies, historial).
    pub privacy: bool,
    /// Analizar el almacén de componentes con DISM (lento, opcional).
    pub component_store: bool,
}

impl ScanContext {
    pub fn new(env: Env, is_admin: bool) -> Self {
        let dev_roots = default_dev_roots(&env);
        Self {
            env,
            cancel: CancelToken::new(),
            is_admin,
            dev_roots,
            exclusions: Vec::new(),
            running: HashSet::new(),
            privacy: false,
            component_store: false,
        }
    }

    pub fn with_dev_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.dev_roots = roots;
        self
    }

    pub fn with_running(mut self, running: HashSet<String>) -> Self {
        self.running = running
            .into_iter()
            .map(|p| p.to_ascii_lowercase())
            .collect();
        self
    }

    /// Aplica los ajustes del usuario (carpetas de proyectos y exclusiones).
    pub fn with_settings(mut self, settings: &Settings) -> Self {
        if !settings.dev_roots.is_empty() {
            self.dev_roots = settings.dev_roots.clone();
        }
        self.exclusions = settings.exclusions.clone();
        self.privacy = settings.privacy;
        self.component_store = settings.component_store;
        self
    }

    /// `true` si `path` es una exclusión o está dentro de una.
    pub fn is_excluded(&self, path: &Path) -> bool {
        self.exclusions.iter().any(|e| path_within(path, e))
    }
}

/// `child` es igual a `parent` o está dentro (sin distinguir mayúsculas ni `/`/`\`).
pub fn path_within(child: &Path, parent: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().replace('/', "\\").to_lowercase();
        s.trim_end_matches('\\').to_owned()
    };
    let (c, p) = (norm(child), norm(parent));
    !p.is_empty() && (c == p || c.starts_with(&format!("{p}\\")))
}

/// Carpetas típicas de proyectos bajo el perfil del usuario (sólo las que existen).
pub fn default_dev_roots(env: &Env) -> Vec<PathBuf> {
    const NAMES: &[&str] = &[
        "Documents",
        "source",
        "repos",
        "projects",
        "Projects",
        "dev",
        "code",
        "src",
        "workspace",
        "Github",
        "GitHub",
        "git",
        "Desktop",
    ];
    let mut roots: Vec<PathBuf> = NAMES
        .iter()
        .map(|n| env.home.join(n))
        .filter(|p| p.is_dir())
        .collect();
    // En Windows las rutas no distinguen mayúsculas: quitar duplicados y anidadas.
    roots.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    roots.dedup_by_key(|p| p.to_string_lossy().to_lowercase());
    let snapshot = roots.clone();
    roots.retain(|p| !snapshot.iter().any(|q| q != p && path_within(p, q)));
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within() {
        assert!(path_within(Path::new(r"C:\A\b\c"), Path::new(r"c:\a\B\")));
        assert!(path_within(Path::new(r"C:\a\b"), Path::new(r"C:\a\b")));
        assert!(!path_within(Path::new(r"C:\a\bc"), Path::new(r"C:\a\b")));
        assert!(!path_within(Path::new(r"C:\a"), Path::new("")));
    }
}
