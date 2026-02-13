//! Core engine for directory walking and file processing.

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

/// Internal walker that integrates ignore rules and glob patterns.
struct Walker {
    inner: ignore::Walk,
    #[allow(dead_code)]
    matcher: Option<globset::GlobSet>,
}

impl Walker {
    /// Creates a new Walker based on the given options.
    fn new(options: &SnapcatOptions) -> Result<Self, SnapcatError> {
        let mut builder = WalkBuilder::new(&options.root);
        builder
            .git_ignore(options.respect_gitignore)
            .hidden(!options.include_hidden)
            .max_depth(options.max_depth)
            .follow_links(options.follow_links)
            .ignore(false); // we handle ignore patterns ourselves

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

    /// Converts the walker into an iterator over paths.
    fn into_iter(self) -> impl Iterator<Item = Result<PathBuf, SnapcatError>> {
        self.inner.filter_map(|result| match result {
            Ok(entry) => Some(Ok(entry.path().to_path_buf())),
            Err(e) => Some(Err(SnapcatError::Walk(e.to_string()))),
        })
    }

    /// Collects all paths into a Vec.
    fn collect_entries(self) -> Result<Vec<PathBuf>, SnapcatError> {
        self.into_iter().collect()
    }
}

/// Reads a file's content with binary detection and size limit.
///
/// Returns a tuple `(content, is_binary)`.
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

    // Read first 4KiB for binary detection
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

/// Main entry point for a snapcat operation.
///
/// This function walks the directory tree starting at `options.root`, collects all files,
/// reads their contents (subject to size limits and binary detection), and returns a
/// [`SnapcatResult`] containing the tree representation and file entries.
///
/// # Errors
///
/// Returns an error if the directory walk fails, if file I/O fails, or if glob patterns are invalid.
///
/// # Example
///
/// ```
/// use snapcat::{SnapcatBuilder, snapcat};
///
/// let options = SnapcatBuilder::new(".").build();
/// let result = snapcat(options).expect("snapcat failed");
/// println!("{}", result.tree);
/// ```
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

/// Process files sequentially.
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

/// Process files in parallel using Rayon.
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

/// A streaming iterator over file entries.
///
/// This allows processing files one by one without loading all into memory at once.
/// Only available when the `streaming` feature is enabled.
#[cfg(feature = "streaming")]
pub struct SnapcatStream {
    path_iter: Box<dyn Iterator<Item = Result<PathBuf, SnapcatError>> + Send>,
    options: SnapcatOptions,
}

#[cfg(feature = "streaming")]
impl SnapcatStream {
    /// Creates a new streaming iterator for the given options.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory walker cannot be created (e.g., invalid patterns).
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

    /// Returns the next file entry, or `None` if the iteration is complete.
    ///
    /// Each item is a `Result` that may contain an error if reading that particular file fails.
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
