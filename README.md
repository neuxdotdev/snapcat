# Snapcat

[![Crates.io](https://img.shields.io/crates/v/snapcat)](https://crates.io/crates/snapcat)
[![Docs.rs](https://docs.rs/snapcat/badge.svg)](https://docs.rs/snapcat)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-red.svg)](LICENSE)

**Snapcat** is a Rust library for traversing directories, building a visual directory tree, and collecting file contents with optional binary detection, size limits, and glob-based ignore patterns. It’s designed for tools that need to snapshot or analyze project structures (e.g., code summarizers, backup tools, or static analyzers).

## Features

- **Directory Walking**: Uses the efficient `ignore` crate with support for `.gitignore`, hidden files, symlinks, and depth limits.
- **Custom Ignore Patterns**: Exclude files/folders using glob patterns (e.g., `*.log`, `target/*`).
- **Binary Detection**: Automatically detect binary files (simple null-byte check or accurate content inspection) and omit their content.
- **File Size Limits**: Skip content for files larger than a given threshold.
- **Directory Tree**: Generates a human-readable ASCII tree of the visited structure.
- **Parallel Processing**: Optional parallel file reading via Rayon (feature `parallel`).
- **Streaming**: Process files one by one without loading everything into memory (feature `streaming`).
- **Optional Logging**: Integrates with `tracing` for debug output (feature `logging`).
- **Serde Support**: All public types implement `Serialize` for easy JSON/YAML output.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
snapcat = "0.1.1"
```

To enable optional features:

```toml
[dependencies]
snapcat = { version = "0.1.1", features = ["parallel", "streaming", "logging"] }
```

Available features:

| Feature     | Description                                           |
| ----------- | ----------------------------------------------------- |
| `parallel`  | Use Rayon for concurrent file processing.             |
| `streaming` | Enable `SnapcatStream` for iterator‑based processing. |
| `logging`   | Enable `tracing` debug logs (useful for debugging).   |

## Quick Start

```rust
use snapcat::{SnapcatBuilder, snapcat};

fn main() -> Result<(), snapcat::SnapcatError> {
    let options = SnapcatBuilder::new(".")
        .respect_gitignore(true)
        .include_hidden(false)
        .ignore_patterns(vec!["*.tmp".to_string()])
        .build();

    let result = snapcat(options)?;

    // Print directory tree
    println!("{}", result.tree);

    // Print file paths and their contents
    for file in result.files {
        println!("File: {}", file.path.display());
        if !file.is_binary {
            println!("{}", file.content);
        }
    }

    Ok(())
}
```

## Configuration

All options are configured via `SnapcatBuilder` or directly via `SnapcatOptions`. The builder provides a fluent interface:

```rust
use snapcat::{SnapcatBuilder, BinaryDetection};

let options = SnapcatBuilder::new("./src")
    .respect_gitignore(false)
    .max_depth(5)
    .include_hidden(true)
    .follow_links(true)
    .ignore_patterns(vec!["node_modules".to_string(), "*.log".to_string()])
    .file_size_limit(Some(1024 * 1024)) // 1 MB
    .binary_detection(BinaryDetection::Accurate)
    .include_file_size(true)
    .build();
```

### `BinaryDetection`

Controls how binary files are identified.

| Variant    | Description                                                              |
| ---------- | ------------------------------------------------------------------------ |
| `Simple`   | Checks for a null byte in the first 4 KB (fast, usually sufficient).     |
| `Accurate` | Uses the `content_inspector` crate for more accurate detection (slower). |
| `None`     | Treats all files as text.                                                |

### `SnapcatOptions`

| Field               | Type              | Description                                               |
| ------------------- | ----------------- | --------------------------------------------------------- |
| `root`              | `PathBuf`         | Starting directory.                                       |
| `respect_gitignore` | `bool`            | Honor `.gitignore` files.                                 |
| `max_depth`         | `Option<usize>`   | Maximum recursion depth (`None` = unlimited).             |
| `include_hidden`    | `bool`            | Include hidden files and directories (starting with `.`). |
| `follow_links`      | `bool`            | Follow symbolic links.                                    |
| `ignore_patterns`   | `Vec<String>`     | Glob patterns to exclude (e.g., `"*.log"`).               |
| `file_size_limit`   | `Option<u64>`     | Skip content for files larger than this (bytes).          |
| `binary_detection`  | `BinaryDetection` | Method to detect binary files.                            |
| `include_file_size` | `bool`            | Include file size in `FileEntry`.                         |

## Output

### `SnapcatResult`

Returned by `snapcat()`:

- `tree: String` – ASCII directory tree (like `tree` command).
- `files: Vec<FileEntry>` – List of processed files.

### `FileEntry`

| Field       | Type          | Description                                          |
| ----------- | ------------- | ---------------------------------------------------- |
| `path`      | `PathBuf`     | Absolute or relative path (as given by walker).      |
| `content`   | `String`      | File contents or placeholder message.                |
| `is_binary` | `bool`        | Whether the file was detected as binary.             |
| `size`      | `Option<u64>` | File size in bytes (if `include_file_size` is true). |

When a file is skipped because it’s too large, `content` becomes `"[File too large, content omitted]"` and `is_binary` is `false`.  
For binary files, `content` becomes `"[Binary file, content omitted]"` and `is_binary` is `true`.

## Streaming Mode

For very large directories, you can process files one by one without holding all results in memory. Enable the `streaming` feature and use `SnapcatStream`:

```rust
use snapcat::{SnapcatBuilder, SnapcatStream};

let options = SnapcatBuilder::new("/var/log").build();
let mut stream = SnapcatStream::new(options)?;

for item in stream {
    match item {
        Ok(entry) => println!("{} ({} bytes)", entry.path.display(), entry.size.unwrap_or(0)),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Parallel Processing

When the `parallel` feature is enabled, `snapcat()` automatically processes files using Rayon’s thread pool. This can significantly speed up reading many small files. No code changes are required—just enable the feature.

## Logging

If the `logging` feature is active, Snapcat emits `tracing` debug events. You can capture them by installing a `tracing` subscriber:

```rust
use tracing_subscriber;

tracing_subscriber::fmt::init();

// Now snapcat will log debug information
let result = snapcat(options)?;
```

## Error Handling

All fallible operations return `SnapcatError`, which implements `std::error::Error`. Variants include:

- `Io { path, source }` – I/O error on a specific file.
- `Walk(String)` – Error while walking the directory (e.g., permission denied).
- `InvalidPath(String)` – The provided root path is invalid.
- `BinaryDetection` – (rare) binary detection failure.

## Examples

### Dump all Rust files

```rust
let options = SnapcatBuilder::new(".").build();
let result = snapcat(options)?;

for file in result.files {
    if file.path.extension().map_or(false, |ext| ext == "rs") {
        println!("// {}", file.path.display());
        println!("{}", file.content);
    }
}
```

### Generate a JSON snapshot

```rust
let options = SnapcatBuilder::new(".").include_file_size(true).build();
let result = snapcat(options)?;
let json = serde_json::to_string_pretty(&result)?;
std::fs::write("snapshot.json", json)?;
```

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

See the [LICENSE](LICENSE) file for details.

## Contribution

Contributions are welcome! Please open an issue or PR on GitHub.
