//! Configuration options for directory walking and file processing.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Method used to detect whether a file is binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryDetection {
    /// Simple detection: check for null bytes in the first 4 KiB of the file.
    Simple,
    /// More accurate detection using the `content_inspector` crate.
    Accurate,
    /// No binary detection; all files are treated as text.
    None,
}

/// Configuration options for a snapcat operation.
///
/// This struct can be constructed directly or via the [`SnapcatBuilder`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapcatOptions {
    /// The root directory to start walking from.
    pub root: PathBuf,
    /// Whether to respect `.gitignore` files.
    pub respect_gitignore: bool,
    /// Maximum depth to walk (None means unlimited).
    pub max_depth: Option<usize>,
    /// Whether to include hidden files and directories (those starting with a dot).
    pub include_hidden: bool,
    /// Whether to follow symbolic links.
    pub follow_links: bool,
    /// List of glob patterns to ignore.
    pub ignore_patterns: Vec<String>,
    /// Maximum file size (in bytes) to read; files larger than this will have content omitted.
    pub file_size_limit: Option<u64>,
    /// Method used to detect binary files.
    pub binary_detection: BinaryDetection,
    /// Whether to include file size in the output.
    pub include_file_size: bool,
}

impl Default for SnapcatOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            respect_gitignore: true,
            max_depth: None,
            include_hidden: false,
            follow_links: false,
            ignore_patterns: Vec::new(),
            file_size_limit: None,
            binary_detection: BinaryDetection::Simple,
            include_file_size: false,
        }
    }
}

/// A builder for constructing [`SnapcatOptions`] with a fluent interface.
#[derive(Debug, Default)]
pub struct SnapcatBuilder {
    options: SnapcatOptions,
}

impl SnapcatBuilder {
    /// Creates a new builder with the given root directory.
    ///
    /// # Example
    ///
    /// ```
    /// use snapcat::SnapcatBuilder;
    ///
    /// let builder = SnapcatBuilder::new("/path/to/dir");
    /// ```
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            options: SnapcatOptions {
                root: root.into(),
                ..Default::default()
            },
        }
    }

    /// Sets whether to respect `.gitignore` files.
    pub fn respect_gitignore(mut self, yes: bool) -> Self {
        self.options.respect_gitignore = yes;
        self
    }

    /// Sets the maximum depth to walk.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.options.max_depth = Some(depth);
        self
    }

    /// Removes the depth limit (equivalent to `max_depth(None)`).
    pub fn no_limit_depth(mut self) -> Self {
        self.options.max_depth = None;
        self
    }

    /// Sets whether to include hidden files and directories.
    pub fn include_hidden(mut self, yes: bool) -> Self {
        self.options.include_hidden = yes;
        self
    }

    /// Sets whether to follow symbolic links.
    pub fn follow_links(mut self, yes: bool) -> Self {
        self.options.follow_links = yes;
        self
    }

    /// Sets the list of glob patterns to ignore.
    ///
    /// Patterns are matched against the full path. Example: `"*.tmp"`, `"build/*"`.
    pub fn ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.options.ignore_patterns = patterns;
        self
    }

    /// Sets the maximum file size (in bytes) to read.
    ///
    /// Files larger than this will have their content replaced with an omission message.
    pub fn file_size_limit(mut self, limit: Option<u64>) -> Self {
        self.options.file_size_limit = limit;
        self
    }

    /// Sets the binary detection method.
    pub fn binary_detection(mut self, method: BinaryDetection) -> Self {
        self.options.binary_detection = method;
        self
    }

    /// Sets whether to include file size in the output.
    pub fn include_file_size(mut self, yes: bool) -> Self {
        self.options.include_file_size = yes;
        self
    }

    /// Builds the final [`SnapcatOptions`].
    pub fn build(self) -> SnapcatOptions {
        self.options
    }
}
