use crp_core::{AppPaths, CancelToken, Engine, Finding, ScanContext, Settings};
use crp_disk::dupes::DupeReport;
use crp_disk::tree::DiskTree;
use std::sync::{Mutex, MutexGuard};

/// Aplica el idioma elegido (o el del sistema) a los textos del motor.
pub fn apply_language(settings: &Settings) -> &'static str {
    let wanted = settings
        .language
        .clone()
        .or_else(crp_platform::locale::ui_language)
        .unwrap_or_default();
    crp_core::i18n::set_language(&wanted)
}

/// Estado compartido de la app.
///
/// El último escaneo se guarda aquí: la UI sólo envía **ids** para limpiar y
/// el backend usa sus propios hallazgos. Así el frontend nunca decide rutas.
pub struct AppState {
    /// Se reconstruye al cambiar de idioma (nombres y descripciones).
    pub engine: Mutex<Engine>,
    pub paths: AppPaths,
    pub settings: Mutex<Settings>,
    pub last_scan: Mutex<Vec<Finding>>,
    pub cancel: Mutex<CancelToken>,
    /// Último análisis de espacio (árbol en memoria).
    pub disk: Mutex<Option<DiskTree>>,
    pub disk_cancel: Mutex<CancelToken>,
    /// Última búsqueda de duplicados.
    pub dupes: Mutex<Option<DupeReport>>,
    pub dupes_cancel: Mutex<CancelToken>,
}

impl AppState {
    pub fn new() -> Self {
        let paths = crp_modules::app_paths();
        // Ajustes ilegibles → valores por defecto (no impedir que la app abra).
        let settings = Settings::load(&paths.settings_file()).unwrap_or_default();
        apply_language(&settings);
        Self {
            engine: Mutex::new(crp_modules::engine()),
            paths,
            settings: Mutex::new(settings),
            last_scan: Mutex::new(Vec::new()),
            cancel: Mutex::new(CancelToken::new()),
            disk: Mutex::new(None),
            disk_cancel: Mutex::new(CancelToken::new()),
            dupes: Mutex::new(None),
            dupes_cancel: Mutex::new(CancelToken::new()),
        }
    }

    /// Contexto con los ajustes actuales y un token de cancelación nuevo.
    pub fn fresh_context(&self) -> ScanContext {
        let ctx = crp_modules::system_context().with_settings(&lock(&self.settings));
        *lock(&self.cancel) = ctx.cancel.clone();
        ctx
    }

    /// Token nuevo para una tarea; el anterior queda cancelable desde `cancel`.
    pub fn fresh_token(slot: &Mutex<CancelToken>) -> CancelToken {
        let t = CancelToken::new();
        *lock(slot) = t.clone();
        t
    }
}

/// `lock()` que no propaga el envenenamiento: un pánico en otro hilo no debe
/// dejar la app inutilizable (los datos protegidos siguen siendo coherentes).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
