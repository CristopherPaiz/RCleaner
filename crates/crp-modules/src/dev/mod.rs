//! Herramientas de desarrollo: cachés de gestores de paquetes, editores y
//! artefactos de compilación dentro de proyectos.

mod caches;
mod projects;

use crp_core::Engine;

pub fn register(e: &mut Engine) {
    caches::register(e);
    e.register(projects::Projects::new());
}
