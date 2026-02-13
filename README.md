# Snapcat

[![Crates.io](https://img.shields.io/crates/v/snapcat)](https://crates.io/crates/snapcat)
[![Docs.rs](https://docs.rs/snapcat/badge.svg)](https://docs.rs/snapcat)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

**Snapcat** is a fast, flexible Rust library for walking directories, generating a visual tree, and collecting file contents with intelligent filtering. It’s perfect for code summarizers, backup tools, static analyzers, or any project that needs a structured snapshot of a filesystem.

## Features

- **Efficient directory walking** – powered by the [`ignore`](https://crates.io/crates/ignore) crate, with `.gitignore` support, hidden file control, symlink following, and depth limits.
- **Glob-based ignore patterns** – exclude files/folders using familiar patterns like `*.log` or `target/`.
- **Binary detection** – automatically skip binary files with simple null‑byte check or accurate content inspection.
- **File size limits** – omit content for files larger than a threshold.
- **ASCII directory tree** – get a human‑readable tree like the `tree` command.
- **Parallel processing** (optional) – use Rayon to read files concurrently.
- **Streaming mode** (optional) – process files one‑by‑one without loading everything into memory.
- **Structured output** – `SnapcatResult` and `FileEntry` implement `serde::Serialize` / `Deserialize` for easy JSON, YAML, etc.
- **Flexible formatting** – built‑in `output` module provides Markdown, plain text, and JSON formatters.
- **Optional logging** – integrate with `tracing` for debug output.

## Installation

Or use `cargo add`:

```sh
cargo add snapcat
```

### Enable optional features

```sh
cargo add snapcat --features parallel,streaming,logging
```

| Feature     | Description                                       |
| ----------- | ------------------------------------------------- |
| `parallel`  | Parallel file reading with Rayon.                 |
| `streaming` | Iterator‑based processing (low memory footprint). |
| `logging`   | `tracing` debug logs (useful for debugging).      |

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

    // Print the directory tree
    println!("{}", result.tree);

    // Print each file's path and content
    for file in result.files {
        println!("File: {}", file.path.display());
        if !file.is_binary {
            println!("{}", file.content);
        }
    }

    Ok(())
}
```

## ️ Configuration

All options are configured via `SnapcatBuilder` (or directly via `SnapcatOptions`). The builder provides a fluent interface:

```rust
use snapcat::{SnapcatBuilder, BinaryDetection};

let options = SnapcatBuilder::new("./src")
    .respect_gitignore(false)
    .max_depth(5)
    .include_hidden(true)
    .follow_links(true)
    .ignore_patterns(vec!["node_modules".into(), "*.log".into()])
    .file_size_limit(Some(1024 * 1024))   // 1 MB
    .binary_detection(BinaryDetection::Accurate)
    .include_file_size(true)
    .build();
```

### `BinaryDetection`

| Variant    | Description                                                                                        |
| ---------- | -------------------------------------------------------------------------------------------------- |
| `Simple`   | Checks for null bytes in the first 4 KiB (fast, usually sufficient).                               |
| `Accurate` | Uses [`content_inspector`](https://crates.io/crates/content_inspector) (slower but more accurate). |
| `None`     | Treats all files as text.                                                                          |

### `SnapcatOptions`

| Field               | Type              | Description                                      |
| ------------------- | ----------------- | ------------------------------------------------ |
| `root`              | `PathBuf`         | Starting directory.                              |
| `respect_gitignore` | `bool`            | Honor `.gitignore` files.                        |
| `max_depth`         | `Option<usize>`   | Maximum recursion depth (`None` = unlimited).    |
| `include_hidden`    | `bool`            | Include hidden files/dirs (starting with `.`).   |
| `follow_links`      | `bool`            | Follow symbolic links.                           |
| `ignore_patterns`   | `Vec<String>`     | Glob patterns to exclude (e.g., `"*.log"`).      |
| `file_size_limit`   | `Option<u64>`     | Skip content for files larger than this (bytes). |
| `binary_detection`  | `BinaryDetection` | Method to detect binary files.                   |
| `include_file_size` | `bool`            | Include file size in `FileEntry`.                |

## Output

### `SnapcatResult`

Returned by `snapcat()`:

- `tree: String` – ASCII directory tree.
- `files: Vec<FileEntry>` – List of processed files.

### `FileEntry`

| Field       | Type          | Description                                          |
| ----------- | ------------- | ---------------------------------------------------- |
| `path`      | `PathBuf`     | Path to the file.                                    |
| `content`   | `String`      | File contents or placeholder message.                |
| `is_binary` | `bool`        | Whether the file was detected as binary.             |
| `size`      | `Option<u64>` | File size in bytes (if `include_file_size` is true). |

When a file is skipped because it’s too large, `content` becomes `"[File too large, content omitted]"` and `is_binary` is `false`.  
For binary files, `content` becomes `"[Binary file, content omitted]"` and `is_binary` is `true`.

## Advanced Features

### Streaming Mode

Process files one by one without holding all results in memory – ideal for huge directories.

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

### Parallel Processing

Enable the `parallel` feature and `snapcat()` will automatically use Rayon’s thread pool to read files concurrently – no code changes required.

### Logging

With the `logging` feature, snapcat emits `tracing` debug events. Install a subscriber to see them:

```rust
use tracing_subscriber;

tracing_subscriber::fmt::init();

// Now snapcat will log debug info
let result = snapcat(options)?;
```

### Output Formatting

The `output` module provides helpers to format results as Markdown, plain text, or JSON, and write them to files.

```rust
use snapcat::{SnapcatBuilder, snapcat, output::{OutputFormat, write_result_to_file}};

let result = snapcat(SnapcatBuilder::new(".").build())?;
write_result_to_file(&result, OutputFormat::Markdown, "snapshot.md", true)?;
```

## ️ Error Handling

All fallible operations return `SnapcatError`, which implements `std::error::Error`.

```rust
pub enum SnapcatError {
    Io { path: PathBuf, source: std::io::Error },
    Walk(String),
    InvalidPath(String),
    BinaryDetection,
}
```

- `Io` – I/O error on a specific file (includes the path).
- `Walk` – Error while walking the directory (e.g., permission denied).
- `InvalidPath` – The root path is invalid.
- `BinaryDetection` – (Rare) binary detection failure.

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

This project is licensed under the **GNU Affero General Public License v3.0** – see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Feel free to open an issue or pull request on [GitHub](https://github.com/your-repo/snapcat).
