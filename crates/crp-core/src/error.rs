use std::path::PathBuf;

/// Errores del motor.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("E/S en {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("ruta protegida, no se puede limpiar: {0}")]
    Protected(PathBuf),

    #[error("se requieren permisos de administrador")]
    RequiresAdmin,

    #[error("operación cancelada")]
    Cancelled,

    #[error("el módulo '{0}' no implementa limpieza personalizada")]
    Unsupported(String),

    #[error("módulo desconocido: {0}")]
    UnknownModule(String),

    #[error("el comando '{program}' falló: {message}")]
    Command { program: String, message: String },

    #[error("configuración inválida: {0}")]
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
