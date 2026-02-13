use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub content: String,
    pub is_binary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}
#[derive(Debug, Serialize)]
pub struct SnapcatResult {
    pub tree: String,
    pub files: Vec<FileEntry>,
}
