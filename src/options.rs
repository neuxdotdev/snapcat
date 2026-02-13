use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryDetection {
    Simple,
    Accurate,
    None,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapcatOptions {
    pub root: PathBuf,
    pub respect_gitignore: bool,
    pub max_depth: Option<usize>,
    pub include_hidden: bool,
    pub follow_links: bool,
    pub ignore_patterns: Vec<String>,
    pub file_size_limit: Option<u64>,
    pub binary_detection: BinaryDetection,
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
#[derive(Debug, Default)]
pub struct SnapcatBuilder {
    options: SnapcatOptions,
}
impl SnapcatBuilder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            options: SnapcatOptions {
                root: root.into(),
                ..Default::default()
            },
        }
    }
    pub fn respect_gitignore(mut self, yes: bool) -> Self {
        self.options.respect_gitignore = yes;
        self
    }
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.options.max_depth = Some(depth);
        self
    }
    pub fn no_limit_depth(mut self) -> Self {
        self.options.max_depth = None;
        self
    }
    pub fn include_hidden(mut self, yes: bool) -> Self {
        self.options.include_hidden = yes;
        self
    }
    pub fn follow_links(mut self, yes: bool) -> Self {
        self.options.follow_links = yes;
        self
    }
    pub fn ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.options.ignore_patterns = patterns;
        self
    }
    pub fn file_size_limit(mut self, limit: Option<u64>) -> Self {
        self.options.file_size_limit = limit;
        self
    }
    pub fn binary_detection(mut self, method: BinaryDetection) -> Self {
        self.options.binary_detection = method;
        self
    }
    pub fn include_file_size(mut self, yes: bool) -> Self {
        self.options.include_file_size = yes;
        self
    }
    pub fn build(self) -> SnapcatOptions {
        self.options
    }
}
