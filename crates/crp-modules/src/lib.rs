//! # crp-modules
//!
//! Catálogo de limpiadores. Cada submódulo expone `register(&mut Engine)`.
//! Convención de ids: `<categoría>.<nombre>` (`system.*`, `browser.*`, `dev.*`, `ai.*`)
//! para que los modos puedan seleccionarlos con `categoría.*`.
//!
//! Para añadir un limpiador basado en rutas basta un [`crp_core::PathRuleModule`];
//! ver `dev/caches.rs` como ejemplo.
// Todo el código de sistema (`unsafe`/FFI) vive sólo en crp-platform.
#![forbid(unsafe_code)]

pub mod ai;
pub mod browsers;
pub mod dev;
pub mod system;

use crp_core::{AppPaths, Engine, Env, ScanContext, Settings};

/// Registra todos los módulos en un motor existente.
pub fn register_all(engine: &mut Engine) {
    system::register(engine);
    browsers::register(engine);
    dev::register(engine);
    ai::register(engine);
}

/// Motor con todos los módulos incluidos.
pub fn engine() -> Engine {
    let mut e = Engine::new();
    register_all(&mut e);
    e
}

/// Contexto real del sistema actual (sin ajustes de usuario).
pub fn system_context() -> ScanContext {
    ScanContext::new(Env::from_system(), crp_platform::is_elevated())
        .with_running(crp_platform::processes::running())
}

/// Rutas de ajustes/historial/modos del usuario actual.
pub fn app_paths() -> AppPaths {
    AppPaths::new(&Env::from_system())
}

/// Contexto real con los ajustes guardados del usuario (si se pueden leer).
pub fn user_context() -> ScanContext {
    let settings = Settings::load(&app_paths().settings_file()).unwrap_or_default();
    system_context().with_settings(&settings)
}

#[cfg(test)]
mod tests {
    #[test]
    fn ids_follow_convention() {
        let e = super::engine();
        for m in e.metas() {
            let prefix = m.id.split('.').next().unwrap();
            assert!(
                ["system", "browser", "dev", "ai"].contains(&prefix),
                "id sin categoría válida: {}",
                m.id
            );
            assert!(!m.description.is_empty(), "{} sin descripción", m.id);
        }
    }

    #[test]
    fn every_mode_selects_something() {
        let e = super::engine();
        for mode in crp_core::modes::builtin() {
            assert!(
                !e.resolve(&mode.modules).is_empty(),
                "modo vacío: {}",
                mode.id
            );
        }
    }
}
