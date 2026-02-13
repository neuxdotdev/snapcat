//! Output formatting for snapcat results.
//!
//! Flexible and clean formatting for [`SnapcatResult`] into Markdown, plain text, or JSON.

use crate::{SnapcatError, SnapcatResult};
use std::fs;
use std::path::Path;

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Text,
    Json,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Markdown => "md",
            OutputFormat::Text => "txt",
            OutputFormat::Json => "json",
        }
    }
}

/// Formats the snapcat result into a string.
pub fn format_result(result: &SnapcatResult, format: OutputFormat, pretty: bool) -> String {
    match format {
        OutputFormat::Markdown => format_markdown(result),
        OutputFormat::Text => format_text(result),
        OutputFormat::Json => format_json(result, pretty),
    }
}

/// Writes the formatted result to a file.
pub fn write_result_to_file(
    result: &SnapcatResult,
    format: OutputFormat,
    path: impl AsRef<Path>,
    pretty: bool,
) -> Result<(), SnapcatError> {
    fs::write(&path, format_result(result, format, pretty))
        .map_err(|e| SnapcatError::io(path.as_ref(), e))
}

// ----------------------- Internal helpers -----------------------

/// Wrap content in a code block with optional language
fn code_block(content: &str, lang: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("```{}\n", lang));
    s.push_str(content);
    if !content.ends_with('\n') {
        s.push('\n');
    }
    s.push_str("```\n");
    s
}

/// Formats as Markdown with tree and file sections
fn format_markdown(result: &SnapcatResult) -> String {
    let mut out = String::with_capacity(2048);

    // Tree as code block
    out.push_str(&code_block(&result.tree, ""));

    // Files
    for file in &result.files {
        let path_str = file.path.display().to_string();
        let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        out.push_str(&format!("## {}\n\n", path_str));
        out.push_str(&code_block(&file.content, language_from_extension(ext)));
    }

    out
}

/// Formats as plain text with simple separators
fn format_text(result: &SnapcatResult) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("Directory Tree:\n");
    out.push_str(&result.tree);
    if !result.tree.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("\n\nFiles:\n");

    for file in &result.files {
        out.push_str(&format!("\n--- {} ---\n", file.path.display()));
        out.push_str(&file.content);
        if !file.content.ends_with('\n') {
            out.push('\n');
        }
    }

    out
}

/// Formats as JSON, optionally pretty-printed
fn format_json(result: &SnapcatResult, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(result).expect("JSON serialization failed")
    } else {
        serde_json::to_string(result).expect("JSON serialization failed")
    }
}

/// Maps file extensions to Markdown code block languages
fn language_from_extension(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "toml" => "toml",
        "json" => "json",
        "md" | "markdown" => "markdown",
        "txt" => "text",
        "html" | "htm" => "html",
        "css" => "css",
        "js" => "javascript",
        "py" => "python",
        "sh" | "bash" => "bash",
        "yml" | "yaml" => "yaml",
        "xml" => "xml",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "h" => "c",
        "hpp" => "cpp",
        "go" => "go",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "dart" => "dart",
        _ => "",
    }
}
