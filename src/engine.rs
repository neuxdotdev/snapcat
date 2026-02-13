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
