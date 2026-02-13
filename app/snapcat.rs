//! Command-line interface for snapcat.
//!
//! This binary provides access to the snapcat library functionality,
//! walking a directory tree and outputting the result in various formats.

use clap::{Parser, ValueEnum};
#[cfg(feature = "streaming")]
use snapcat::SnapcatStream;
use snapcat::{BinaryDetection, SnapcatBuilder, SnapcatOptions, SnapcatResult, output, snapcat};
#[cfg(feature = "streaming")]
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;

/// snapcat — fast directory snapshot tool
#[derive(Parser)]
#[command(name = "snapcat", version, about, long_about = None)]
struct Cli {
    /// Root directory (default current dir)
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    format: OutputFormat,

    /// Binary detection strategy
    #[arg(long, default_value = "simple", value_parser = parse_binary_detection)]
    binary_detection: BinaryDetection,

    /// Max depth (unlimited if not set)
    #[arg(long)]
    max_depth: Option<usize>,

    /// Ignore patterns (can be repeated)
    #[arg(short = 'I', long = "ignore")]
    ignore_patterns: Vec<String>,

    /// File size limit in bytes (files larger will have content omitted)
    #[arg(long)]
    file_size_limit: Option<u64>,

    /// Pretty output (indented JSON or formatted markdown/text)
    #[arg(short, long)]
    pretty: bool,

    /// Enable color (tree only)
    #[arg(long)]
    color: bool,

    /// Include hidden files
    #[arg(long)]
    hidden: bool,

    /// Follow symlinks
    #[arg(long)]
    follow_links: bool,

    /// Disable .gitignore handling
    #[arg(long)]
    no_gitignore: bool,

    /// Operation mode
    #[arg(long, value_enum, default_value_t = Mode::Normal)]
    mode: Mode,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Mode {
    Normal,
    TreeOnly,
    PathsOnly,
    #[cfg(feature = "streaming")]
    Streaming,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Json,
    Tree,
    Paths,
    Markdown,
    Text,
}

/// Parse string into BinaryDetection enum.
fn parse_binary_detection(s: &str) -> Result<BinaryDetection, String> {
    match s {
        "simple" => Ok(BinaryDetection::Simple),
        "accurate" => Ok(BinaryDetection::Accurate),
        "none" => Ok(BinaryDetection::None),
        _ => Err(format!("invalid binary detection method: {}", s)),
    }
}

impl Cli {
    fn into_options(self) -> (SnapcatOptions, OutputFormat, Mode, bool, bool) {
        let mut builder = SnapcatBuilder::new(self.root)
            .respect_gitignore(!self.no_gitignore)
            .include_hidden(self.hidden)
            .follow_links(self.follow_links)
            .ignore_patterns(self.ignore_patterns)
            .file_size_limit(self.file_size_limit)
            .binary_detection(self.binary_detection);

        builder = if let Some(depth) = self.max_depth {
            builder.max_depth(depth)
        } else {
            builder.no_limit_depth()
        };

        (
            builder.build(),
            self.format,
            self.mode,
            self.pretty,
            self.color,
        )
    }
}

fn main() {
    let cli = Cli::parse();
    let (options, format, _mode, pretty, color) = cli.into_options();

    #[cfg(feature = "streaming")]
    if mode == Mode::Streaming {
        run_streaming(&options, pretty);
        return;
    }

    run_normal(options, format, pretty, color);
}

fn run_normal(options: SnapcatOptions, format: OutputFormat, pretty: bool, color: bool) {
    match snapcat(options) {
        Ok(result) => output_result(&result, format, pretty, color),
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    }
}

#[cfg(feature = "streaming")]
fn run_streaming(options: &SnapcatOptions, pretty: bool) {
    let stream = match SnapcatStream::new(options.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    };

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for entry in stream {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error: {}", e);
                exit(1);
            }
        };

        let json = if pretty {
            serde_json::to_string_pretty(&entry)
        } else {
            serde_json::to_string(&entry)
        }
        .unwrap_or_else(|e| {
            eprintln!("JSON serialization error: {}", e);
            exit(1);
        });

        if writeln!(handle, "{}", json).is_err() {
            eprintln!("Failed to write to stdout");
            exit(1);
        }
    }
}

fn output_result(result: &SnapcatResult, format: OutputFormat, pretty: bool, _color: bool) {
    match format {
        OutputFormat::Json => {
            let json = if pretty {
                serde_json::to_string_pretty(result)
            } else {
                serde_json::to_string(result)
            }
            .unwrap_or_else(|e| {
                eprintln!("JSON serialization error: {}", e);
                exit(1);
            });
            println!("{}", json);
        }
        OutputFormat::Tree => {
            println!("{}", result.tree);
        }
        OutputFormat::Paths => {
            for file in &result.files {
                println!("{}", file.path.display());
            }
        }
        OutputFormat::Markdown => {
            let out = output::format_result(result, output::OutputFormat::Markdown, pretty);
            print!("{}", out);
        }
        OutputFormat::Text => {
            let out = output::format_result(result, output::OutputFormat::Text, pretty);
            print!("{}", out);
        }
    }
}
