use crp_core::{AppPaths, CancelToken, Engine, Finding, ScanContext, Settings};
use crp_disk::dupes::DupeReport;
use crp_disk::tree::DiskTree;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    /// Borrado en curso desde Espacio o Duplicados.
    pub delete_cancel: Mutex<CancelToken>,
    /// La ventana se está cerrando para quedarse en la bandeja (no salir).
    pub to_tray: AtomicBool,
    /// Último análisis en segundo plano.
    pub background: Mutex<Option<BackgroundStatus>>,
    /// Tareas largas en curso (análisis, limpieza, borrado). Mientras haya
    /// alguna, minimizar sólo oculta la ventana: no se destruye.
    pub busy: AtomicUsize,
}

/// Marca una tarea larga mientras vive.
pub struct Busy<'a>(&'a AtomicUsize);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Resultado del último análisis en segundo plano.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct BackgroundStatus {
    /// Segundos desde 1970 (UTC).
    pub timestamp: u64,
    /// Lo que se puede limpiar sin riesgo con el modo por defecto.
    pub bytes: u64,
    pub findings: usize,
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
            delete_cancel: Mutex::new(CancelToken::new()),
            to_tray: AtomicBool::new(false),
            background: Mutex::new(None),
            busy: AtomicUsize::new(0),
        }
    }

    /// Contexto con los ajustes actuales y un token de cancelación nuevo.
    pub fn fresh_context(&self) -> ScanContext {
        let ctx = crp_modules::system_context().with_settings(&lock(&self.settings));
        *lock(&self.cancel) = ctx.cancel.clone();
        ctx
    }

    /// Cuenta una tarea larga hasta que se suelte el valor devuelto.
    pub fn busy(&self) -> Busy<'_> {
        self.busy.fetch_add(1, Ordering::SeqCst);
        Busy(&self.busy)
    }

    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst) > 0
    }

    /// Libera los análisis grandes (árbol de disco y duplicados) al quedarse
    /// en la bandeja: al volver se analiza de nuevo.
    pub fn drop_heavy(&self) {
        *lock(&self.disk) = None;
        *lock(&self.dupes) = None;
        lock(&self.last_scan).shrink_to_fit();
    }

    /// Selectores del modo por defecto (o "rápido").
    pub fn default_selectors(&self) -> Vec<String> {
        let wanted = lock(&self.settings).default_mode.clone();
        let (modes, _) = crp_core::modes::all(&self.paths.modes_dir());
        modes
            .iter()
            .find(|m| Some(&m.id) == wanted.as_ref())
            .or_else(|| modes.iter().find(|m| m.id == "rapido"))
            .or(modes.first())
            .map(|m| m.modules.clone())
            .unwrap_or_default()
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
