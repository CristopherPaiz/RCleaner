//! Tipos de datos compartidos entre motor, módulos, CLI y GUI (todos serializables).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agrupación visual de módulos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    System,
    Browser,
    Dev,
    Ai,
    Apps,
    Registry,
}

impl Category {
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Browser => "browser",
            Self::Dev => "dev",
            Self::Ai => "ai",
            Self::Apps => "apps",
            Self::Registry => "registry",
        }
    }

    pub fn label(self) -> String {
        crate::i18n::t(&format!("category.{}", self.key()))
    }
}

/// Nivel de riesgo de una limpieza. Sólo [`Risk::Safe`] se preselecciona.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    /// Se regenera solo; no se pierde nada del usuario.
    Safe,
    /// Se regenera pero cuesta (re-descargas, re-indexado, re-compilación).
    Moderate,
    /// Datos que el usuario puso ahí a propósito (modelos, entornos virtuales…).
    Advanced,
}

impl Risk {
    pub fn key(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Moderate => "moderate",
            Self::Advanced => "advanced",
        }
    }

    pub fn label(self) -> String {
        crate::i18n::t(&format!("risk.{}", self.key()))
    }
}

/// Filtro aplicado a cada **archivo** al limpiar el contenido de una carpeta.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryFilter {
    /// Patrones glob sobre el **nombre** del archivo (`thumbcache_*.db`). Vacío = todos.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// Antigüedad mínima (segundos) según la fecha de modificación.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_age_secs: Option<u64>,
    /// Profundidad máxima bajo la raíz (1 = sólo hijos directos).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<usize>,
    /// Rutas excluidas por el usuario dentro de la raíz: no se miden ni se borran.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<PathBuf>,
}

impl EntryFilter {
    /// `true` si el filtro no restringe nada (se vacía la carpeta entera).
    pub fn is_everything(&self) -> bool {
        self.names.is_empty()
            && self.min_age_secs.is_none()
            && self.max_depth.is_none()
            && self.exclude.is_empty()
    }
}

/// Qué hacer para limpiar un hallazgo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Borra el contenido de cada raíz (la carpeta raíz se conserva).
    DeleteContents {
        roots: Vec<PathBuf>,
        filter: EntryFilter,
    },
    /// Borra rutas completas (archivos o carpetas).
    DeletePaths { paths: Vec<PathBuf> },
    /// Ejecuta un programa externo (p. ej. `ollama rm modelo`).
    RunCommand { program: String, args: Vec<String> },
    /// Limpieza propia del módulo (p. ej. vaciar la papelera vía Win32).
    Custom { key: String },
}

/// Algo que se puede limpiar, con su tamaño estimado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Único en un escaneo: `<module_id>:<clave>`.
    pub id: String,
    pub module_id: String,
    pub category: Category,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Ruta representativa para mostrar/abrir en el explorador.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub bytes: u64,
    pub items: u64,
    pub risk: Risk,
    pub requires_admin: bool,
    /// Preselección sugerida para la UI.
    pub selected: bool,
    /// Días desde la última modificación/uso (proyectos, modelos…), si se conoce.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_days: Option<u64>,
    /// Aviso para el usuario (p. ej. "Chrome está abierto").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleError {
    pub module_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanReport {
    pub findings: Vec<Finding>,
    pub errors: Vec<ModuleError>,
    pub elapsed_ms: u128,
}

impl ScanReport {
    pub fn total_bytes(&self) -> u64 {
        self.findings.iter().map(|f| f.bytes).sum()
    }

    pub fn selected_bytes(&self) -> u64 {
        self.findings
            .iter()
            .filter(|f| f.selected)
            .map(|f| f.bytes)
            .sum()
    }
}

/// Resultado de limpiar un hallazgo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanOutcome {
    pub finding_id: String,
    pub freed_bytes: u64,
    pub removed_items: u64,
    /// Archivos que no se pudieron borrar (en uso, sin permisos…).
    pub skipped_items: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanReport {
    pub outcomes: Vec<CleanOutcome>,
    pub dry_run: bool,
    pub elapsed_ms: u128,
}

impl CleanReport {
    pub fn freed_bytes(&self) -> u64 {
        self.outcomes.iter().map(|o| o.freed_bytes).sum()
    }

    pub fn skipped_items(&self) -> u64 {
        self.outcomes.iter().map(|o| o.skipped_items).sum()
    }
}
