use std::path::PathBuf;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum SnapcatError {
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Walk error: {0}")]
    Walk(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Binary detection failed")]
    BinaryDetection,
}
impl SnapcatError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        SnapcatError::Io {
            path: path.into(),
            source,
        }
    }
}
