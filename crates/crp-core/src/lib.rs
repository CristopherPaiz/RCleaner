//! # crp-core
//!
//! Núcleo de RCleaner. No sabe nada de rutas concretas de Windows ni de la UI:
//! define el **modelo** (hallazgos, acciones, riesgos), el **contrato** de los
//! módulos ([`CleanerModule`]), el **motor** que los ejecuta en paralelo
//! ([`Engine`]) y las **guardas de seguridad** que impiden borrar rutas críticas.
//!
//! Flujo típico:
//!
//! 1. `crp-modules` registra los módulos en un [`Engine`].
//! 2. La UI/CLI elige un [`Mode`] (lista de selectores de módulos).
//! 3. [`Engine::scan`] devuelve un [`ScanReport`] con [`Finding`]s (nada se borra).
//! 4. El usuario confirma y [`Engine::clean`] ejecuta las [`Action`]s elegidas.
// Todo el código de sistema (`unsafe`/FFI) vive sólo en crp-platform.
#![forbid(unsafe_code)]

pub mod context;
pub mod engine;
pub mod env;
pub mod error;
pub mod exec;
pub mod fsutil;
pub mod i18n;
pub mod model;
pub mod modes;
pub mod module;
pub mod progress;
pub mod rules;
pub mod safety;
pub mod settings;

pub use context::{CancelToken, ScanContext, path_within};
pub use engine::{CleanOptions, Engine};
pub use env::Env;
pub use error::{Error, Result};
pub use i18n::{t, tf, tn};
pub use model::{
    Action, Category, CleanOutcome, CleanReport, EntryFilter, Finding, ModuleError, Risk,
    ScanReport,
};
pub use modes::Mode;
pub use module::{CleanerModule, ModuleMeta};
pub use progress::{NoProgress, ProgressEvent, ProgressSink};
pub use rules::{PathRuleModule, Target};
pub use settings::{AppPaths, Background, DeletionRecord, HistoryEntry, Settings};
