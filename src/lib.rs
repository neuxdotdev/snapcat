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
