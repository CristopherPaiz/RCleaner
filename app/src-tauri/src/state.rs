use crp_core::{AppPaths, CancelToken, Engine, Finding, ScanContext, Settings};
use std::sync::{Mutex, MutexGuard};

/// Estado compartido de la app.
///
/// El último escaneo se guarda aquí: la UI sólo envía **ids** para limpiar y
/// el backend usa sus propios hallazgos. Así el frontend nunca decide rutas.
pub struct AppState {
    pub engine: Engine,
    pub paths: AppPaths,
    pub settings: Mutex<Settings>,
    pub last_scan: Mutex<Vec<Finding>>,
    pub cancel: Mutex<CancelToken>,
}

impl AppState {
    pub fn new() -> Self {
        let paths = crp_modules::app_paths();
        // Ajustes ilegibles → valores por defecto (no impedir que la app abra).
        let settings = Settings::load(&paths.settings_file()).unwrap_or_default();
        Self {
            engine: crp_modules::engine(),
            paths,
            settings: Mutex::new(settings),
            last_scan: Mutex::new(Vec::new()),
            cancel: Mutex::new(CancelToken::new()),
        }
    }

    /// Contexto con los ajustes actuales y un token de cancelación nuevo.
    pub fn fresh_context(&self) -> ScanContext {
        let ctx = crp_modules::system_context().with_settings(&lock(&self.settings));
        *lock(&self.cancel) = ctx.cancel.clone();
        ctx
    }
}

/// `lock()` que no propaga el envenenamiento: un pánico en otro hilo no debe
/// dejar la app inutilizable (los datos protegidos siguen siendo coherentes).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
