use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single file entry with its path, content, and metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    /// The full path to the file.
    pub path: PathBuf,
    /// The content of the file as a string.
    ///
    /// If the file was detected as binary or exceeded the size limit, this will contain
    /// a placeholder message like `[Binary file, content omitted]`.
    pub content: String,
    /// Whether the file was detected as binary.
    pub is_binary: bool,
    /// The size of the file in bytes, if requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// The complete result of a snapcat operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapcatResult {
    /// A visual tree representation of the directory structure.
    ///
    /// This is a string similar to the output of the `tree` command.
    pub tree: String,
    /// A list of all files found, with their content and metadata.
    pub files: Vec<FileEntry>,
}
