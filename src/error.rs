//! Error types for the snapcat crate.

use std::path::PathBuf;
use thiserror::Error;

/// The error type for operations in snapcat.
#[derive(Debug, Error)]
pub enum SnapcatError {
    /// An I/O error occurred while accessing a specific path.
    #[error("I/O error on {path}: {source}")]
    Io {
        /// The path where the I/O error occurred.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// An error occurred while walking the directory tree.
    ///
    /// This typically wraps errors from the `ignore` crate.
    #[error("Walk error: {0}")]
    Walk(String),

    /// The provided path is invalid (e.g., malformed or not accessible).
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Binary detection failed for some reason (should not happen under normal circumstances).
    #[error("Binary detection failed")]
    BinaryDetection,
}

impl SnapcatError {
    /// Creates a new `SnapcatError::Io` from a path and an I/O error.
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        SnapcatError::Io {
            path: path.into(),
            source,
        }
    }
}
