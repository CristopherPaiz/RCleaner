//! Contrato que implementa cada limpiador.

use crate::context::ScanContext;
use crate::error::{Error, Result};
use crate::i18n::t;
use crate::model::{Category, CleanOutcome, Finding, Risk};
use serde::{Deserialize, Serialize};

/// Descripción estática de un módulo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMeta {
    /// Identificador estable con prefijo de categoría: `system.user_temp`, `dev.npm`…
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: Category,
    /// Riesgo por defecto de sus hallazgos.
    pub risk: Risk,
    pub requires_admin: bool,
    /// Ejecutables que bloquean sus archivos (`chrome.exe`). Si alguno está
    /// abierto, el motor añade un aviso a los hallazgos del módulo.
    #[serde(default)]
    pub processes: Vec<String>,
}

impl ModuleMeta {
    /// Nombre y descripción se leen de `module.<id>.name` / `module.<id>.desc`
    /// en `locales/*.json` (en el idioma actual).
    pub fn new(id: &str, category: Category) -> Self {
        Self {
            id: id.to_owned(),
            name: t(&format!("module.{id}.name")),
            description: t(&format!("module.{id}.desc")),
            category,
            risk: Risk::Safe,
            requires_admin: false,
            processes: Vec::new(),
        }
    }

    /// Programas que conviene cerrar antes de limpiar.
    pub fn closes(mut self, processes: &[&str]) -> Self {
        self.processes = processes.iter().map(|p| p.to_ascii_lowercase()).collect();
        self
    }

    /// Nombre propio que no se traduce (p. ej. una marca).
    pub fn named(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }

    pub fn describe(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn risk(mut self, risk: Risk) -> Self {
        self.risk = risk;
        self
    }

    pub fn admin(mut self) -> Self {
        self.requires_admin = true;
        self
    }
}

/// Un limpiador. Escanear **nunca** debe modificar el disco.
///
/// La mayoría de módulos sólo devuelven [`Finding`]s con acciones genéricas
/// ([`crate::Action::DeleteContents`], [`crate::Action::DeletePaths`],
/// [`crate::Action::RunCommand`]) y el motor se encarga de ejecutarlas. Sólo
/// los que usan [`crate::Action::Custom`] implementan [`CleanerModule::clean_custom`].
pub trait CleanerModule: Send + Sync {
    fn meta(&self) -> &ModuleMeta;

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<Finding>>;

    fn clean_custom(&self, finding: &Finding, _ctx: &ScanContext) -> Result<CleanOutcome> {
        let _ = finding;
        Err(Error::Unsupported(self.meta().id.clone()))
    }
}
