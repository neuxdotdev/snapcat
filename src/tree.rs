use crate::error::SnapcatError;
use std::path::{Path, PathBuf};
pub fn build_tree_from_entries(root: &Path, entries: &[PathBuf]) -> Result<String, SnapcatError> {
    let mut sorted: Vec<_> = entries.iter().filter(|p| *p != root).collect();
    sorted.sort_by(|a, b| a.components().cmp(b.components()));
    let mut lines = Vec::new();
    lines.push(format!(".  # {}", root.display()));
    for entry in sorted {
        let relative = entry.strip_prefix(root).unwrap_or(entry);
        let depth = relative.components().count();
        let prefix = if depth == 0 {
            String::new()
        } else {
            "│   ".repeat(depth - 1) + "├── "
        };
        let name = relative.file_name().unwrap().to_string_lossy();
        lines.push(format!("{}{}", prefix, name));
    }
    Ok(lines.join("\n"))
}
