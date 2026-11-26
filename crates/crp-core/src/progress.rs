//! Eventos de progreso. La CLI los imprime; la GUI los reenvía al frontend.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressEvent {
    ScanStarted {
        total: usize,
    },
    ModuleStarted {
        module_id: String,
    },
    ModuleFinished {
        module_id: String,
        done: usize,
        total: usize,
        findings: usize,
        bytes: u64,
    },
    CleanStarted {
        total: usize,
    },
    FindingCleaned {
        finding_id: String,
        done: usize,
        total: usize,
        freed_bytes: u64,
    },
}

/// Receptor de eventos. Debe ser `Sync`: se llama desde varios hilos.
pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}

/// Ignora todos los eventos.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn emit(&self, _event: ProgressEvent) {}
}

impl<F> ProgressSink for F
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    fn emit(&self, event: ProgressEvent) {
        self(event)
    }
}
