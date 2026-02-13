# Directory Tree

```
.  // ./src
├── engine.rs
├── error.rs
├── lib.rs
├── options.rs
├── tree.rs
├── types.rs
```

#### `./src/engine.rs`
```rs
use crate::error::SnapcatError;
use crate::options::{BinaryDetection, SnapcatOptions};
use crate::tree::build_tree_from_entries;
use crate::types::{FileEntry, SnapcatResult};
use ignore::WalkBuilder;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
#[cfg(feature = "logging")]
use tracing;
struct Walker {
    inner: ignore::Walk,
    #[allow(dead_code)]
    matcher: Option<globset::GlobSet>,
}
impl Walker {
    fn new(options: &SnapcatOptions) -> Result<Self, SnapcatError> {
        let mut builder = WalkBuilder::new(&options.root);
        builder
            .git_ignore(options.respect_gitignore)
            .hidden(!options.include_hidden)
            .max_depth(options.max_depth)
            .follow_links(options.follow_links)
            .ignore(false);
        let matcher = if !options.ignore_patterns.is_empty() {
            let mut glob_builder = globset::GlobSetBuilder::new();
            for pattern in &options.ignore_patterns {
                let glob = globset::Glob::new(pattern).map_err(|e| {
                    SnapcatError::Walk(format!("Invalid glob pattern '{}': {}", pattern, e))
                })?;
                glob_builder.add(glob);
            }
            Some(
                glob_builder
                    .build()
                    .map_err(|e| SnapcatError::Walk(format!("Failed to build glob set: {}", e)))?,
            )
        } else {
            None
        };
        if let Some(ref matcher) = matcher {
            let matcher = matcher.clone();
            builder.filter_entry(move |entry| !matcher.is_match(entry.path()));
        }
        Ok(Self {
            inner: builder.build(),
            matcher,
        })
    }
    fn into_iter(self) -> impl Iterator<Item = Result<PathBuf, SnapcatError>> {
        self.inner.filter_map(|result| match result {
            Ok(entry) => Some(Ok(entry.path().to_path_buf())),
            Err(e) => Some(Err(SnapcatError::Walk(e.to_string()))),
        })
    }
    fn collect_entries(self) -> Result<Vec<PathBuf>, SnapcatError> {
        self.into_iter().collect()
    }
}
fn read_file_content(
    path: &Path,
    binary_detection: BinaryDetection,
    size_limit: Option<u64>,
) -> Result<(String, bool), SnapcatError> {
    if let Some(limit) = size_limit {
        let metadata = fs::metadata(path).map_err(|e| SnapcatError::io(path, e))?;
        if metadata.len() > limit {
            #[cfg(feature = "logging")]
            tracing::debug!(
                "File too large ({} > {}), skipping content",
                metadata.len(),
                limit
            );
            return Ok(("[File too large, content omitted]".to_string(), false));
        }
    }
    let file = File::open(path).map_err(|e| SnapcatError::io(path, e))?;
    let mut reader = BufReader::new(file);
    let mut first_chunk = Vec::with_capacity(4096);
    let _ = reader
        .by_ref()
        .take(4096)
        .read_to_end(&mut first_chunk)
        .map_err(|e| SnapcatError::io(path, e))?;
    let is_binary = match binary_detection {
        BinaryDetection::Simple => first_chunk.contains(&0),
        BinaryDetection::Accurate => content_inspector::inspect(&first_chunk).is_binary(),
        BinaryDetection::None => false,
    };
    if is_binary {
        #[cfg(feature = "logging")]
        tracing::debug!("Binary file detected: {}", path.display());
        return Ok(("[Binary file, content omitted]".to_string(), true));
    }
    let mut content = String::from_utf8_lossy(&first_chunk).into_owned();
    reader
        .read_to_string(&mut content)
        .map_err(|e| SnapcatError::io(path, e))?;
    Ok((content, false))
}
pub fn snapcat(options: SnapcatOptions) -> Result<SnapcatResult, SnapcatError> {
    #[cfg(feature = "logging")]
    tracing::debug!("Starting snapcat with root: {}", options.root.display());
    let walker = Walker::new(&options)?;
    let all_entries = walker.collect_entries()?;
    let tree = build_tree_from_entries(&options.root, &all_entries)?;
    let file_paths: Vec<PathBuf> = all_entries.into_iter().filter(|p| p.is_file()).collect();
    #[cfg(not(feature = "parallel"))]
    let files = process_files(file_paths, &options)?;
    #[cfg(feature = "parallel")]
    let files = process_files_parallel(file_paths, &options)?;
    Ok(SnapcatResult { tree, files })
}
#[cfg(not(feature = "parallel"))]
fn process_files(
    paths: Vec<PathBuf>,
    options: &SnapcatOptions,
) -> Result<Vec<FileEntry>, SnapcatError> {
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let (content, is_binary) =
            read_file_content(&path, options.binary_detection, options.file_size_limit)?;
        let size = if options.include_file_size {
            Some(
                fs::metadata(&path)
                    .map_err(|e| SnapcatError::io(&path, e))?
                    .len(),
            )
        } else {
            None
        };
        files.push(FileEntry {
            path,
            content,
            is_binary,
            size,
        });
    }
    Ok(files)
}
#[cfg(feature = "parallel")]
fn process_files_parallel(
    paths: Vec<PathBuf>,
    options: &SnapcatOptions,
) -> Result<Vec<FileEntry>, SnapcatError> {
    paths
        .par_iter()
        .map(|path| {
            let (content, is_binary) =
                read_file_content(path, options.binary_detection, options.file_size_limit)?;
            let size = if options.include_file_size {
                Some(
                    fs::metadata(path)
                        .map_err(|e| SnapcatError::io(path, e))?
                        .len(),
                )
            } else {
                None
            };
            Ok(FileEntry {
                path: path.clone(),
                content,
                is_binary,
                size,
            })
        })
        .collect()
}
#[cfg(feature = "streaming")]
pub struct SnapcatStream {
    path_iter: Box<dyn Iterator<Item = Result<PathBuf, SnapcatError>> + Send>,
    options: SnapcatOptions,
}
#[cfg(feature = "streaming")]
impl SnapcatStream {
    pub fn new(options: SnapcatOptions) -> Result<Self, SnapcatError> {
        let walker = Walker::new(&options)?;
        let path_iter = Box::new(walker.into_iter().filter_map(|res| match res {
            Ok(p) if p.is_file() => Some(Ok(p)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        }));
        Ok(Self { path_iter, options })
    }
}
#[cfg(feature = "streaming")]
impl Iterator for SnapcatStream {
    type Item = Result<FileEntry, SnapcatError>;
    fn next(&mut self) -> Option<Self::Item> {
        let path = match self.path_iter.next()? {
            Ok(p) => p,
            Err(e) => return Some(Err(e)),
        };
        let result = (|| {
            let (content, is_binary) = read_file_content(
                &path,
                self.options.binary_detection,
                self.options.file_size_limit,
            )?;
            let size = if self.options.include_file_size {
                Some(
                    fs::metadata(&path)
                        .map_err(|e| SnapcatError::io(&path, e))?
                        .len(),
                )
            } else {
                None
            };
            Ok(FileEntry {
                path,
                content,
                is_binary,
                size,
            })
        })();
        Some(result)
    }
}

```

#### `./src/error.rs`
```rs
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

```

#### `./src/lib.rs`
```rs
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

```

#### `./src/options.rs`
```rs
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

```

#### `./src/tree.rs`
```rs
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

```

#### `./src/types.rs`
```rs
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

```

