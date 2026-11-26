use crate::i18n::{t, tf};
use std::path::PathBuf;

/// Errores del motor.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", tf("error.io", &[("path", &path.display()), ("error", source)]))]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{}", tf("error.protected", &[("path", &.0.display())]))]
    Protected(PathBuf),

    #[error("{}", t("error.requires_admin"))]
    RequiresAdmin,

    #[error("{}", t("error.cancelled"))]
    Cancelled,

    #[error("{}", tf("error.unsupported", &[("id", .0)]))]
    Unsupported(String),

    #[error("{}", tf("error.unknown_module", &[("id", .0)]))]
    UnknownModule(String),

    #[error("{}", tf("error.command", &[("program", program), ("error", message)]))]
    Command { program: String, message: String },

    #[error("{}", tf("error.config", &[("error", .0)]))]
    Config(String),

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
