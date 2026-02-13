//! # Snapcat
//!
//! `snapcat` is a library for recursively walking a directory tree, building a tree representation,
//! and reading the contents of files with options for binary detection, size limits, and more.
//!
//! It provides both a simple blocking API ([`snapcat`]) and a streaming API ([`SnapcatStream`]) when the
//! `streaming` feature is enabled. Parallel file processing is available with the `parallel` feature.
//!
//! # Features
//!
//! - `parallel`: Enables parallel processing of files using Rayon.
//! - `streaming`: Enables a streaming iterator API for processing files one by one.
//! - `logging`: Enables debug logging via the `tracing` crate.
//!
//! # Example
//!
//! ```no_run
//! use snapcat::{SnapcatBuilder, BinaryDetection, snapcat};
//!
//! let options = SnapcatBuilder::new(".")
//!     .respect_gitignore(true)
//!     .include_hidden(false)
//!     .binary_detection(BinaryDetection::Accurate)
//!     .file_size_limit(Some(10 * 1024 * 1024)) // 10 MB
//!     .build();
//!
//! let result = snapcat(options).expect("Failed to scan directory");
//!
//! println!("Directory tree:\n{}", result.tree);
//! for file in result.files {
//!     println!("File: {} (binary: {})", file.path.display(), file.is_binary);
//! }
//! ```

mod engine;
mod error;
mod options;
mod tree;
mod types;

#[cfg(feature = "streaming")]
pub use engine::SnapcatStream;
pub use engine::snapcat;
pub use error::SnapcatError;
pub use options::{BinaryDetection, SnapcatBuilder, SnapcatOptions};
pub use types::{FileEntry, SnapcatResult};
