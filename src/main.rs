// src/main.rs
use arboard::Clipboard;
use clap::Parser;
use content_inspector::ContentType;
use ignore::WalkBuilder;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Optional: The path to the output markdown file. Writes to file instead of stdout.
    #[arg(short, long, conflicts_with = "clipboard")]
    output: Option<PathBuf>,

    /// Optional: Copy the output directly to the system clipboard.
    #[arg(short, long, conflicts_with = "output")]
    clipboard: bool,

    /// Optional: Specify a root directory instead of the current working directory.
    #[arg(short, long)]
    root: Option<PathBuf>,

    /// Include hidden files and directories (those starting with '.').
    #[arg(long)]
    hidden: bool,
}

// Define common lock file names
const LOCK_FILES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "poetry.lock",
    "Gemfile.lock",
    "composer.lock",
    "Pipfile.lock",
    "go.sum",
    "flake.lock",
];

fn main() -> io::Result<()> {
    let args = Args::parse();

    let root_dir = args
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Use stderr for status messages to avoid polluting stdout
    eprintln!("Scanning directory: {}", root_dir.display());

    if args.clipboard {
        // Write to an in-memory byte vector first
        let mut buffer: Vec<u8> = Vec::new();
        generate_markdown(&mut buffer, &root_dir, args.hidden, None)?;

        // Convert the byte vector to a String
        let output_string = String::from_utf8(buffer).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Generated content is not valid UTF-8: {}", e),
            )
        })?;

        match Clipboard::new() {
            Ok(mut clipboard) => {
                // Use the converted string
                if let Err(e) = clipboard.set_text(output_string) {
                    eprintln!("Error copying to clipboard: {}", e);
                    // Convert arboard error to io::Error for consistent return type
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Clipboard error: {}", e),
                    ));
                } else {
                    eprintln!("Output copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error initializing clipboard: {}", e);
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Clipboard init error: {}", e),
                ));
            }
        }
    } else if let Some(output_path) = args.output {
        eprintln!("Outputting to: {}", output_path.display());

        // Canonicalization logic for filtering the output file itself
        let canonical_output_path = if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
            // Create file first to allow canonicalization
            File::create(&output_path)?;
            fs::canonicalize(&output_path).ok() // ok() converts Result to Option
        } else {
            // Handle case where output path has no parent (e.g., just "file.md")
            File::create(&output_path)?;
            fs::canonicalize(&output_path).ok()
        };

        if canonical_output_path.is_none() {
            eprintln!(
                "Warning: Could not canonicalize output path {}. It might be included if inside the scanned directory.",
                output_path.display()
            );
        }

        let output_file_handle = File::create(&output_path)?; // Re-open for writing
        let mut writer = BufWriter::new(output_file_handle);
        generate_markdown(&mut writer, &root_dir, args.hidden, canonical_output_path)?;
        eprintln!("Successfully wrote codebase to {}", output_path.display());
    } else {
        // Default to stdout
        let stdout = io::stdout();
        let mut handle = BufWriter::new(stdout.lock()); // Lock stdout for buffered writing
        generate_markdown(&mut handle, &root_dir, args.hidden, None)?;
        handle.flush()?; // Ensure buffer is flushed before program exits
    }

    Ok(())
}

// Centralized function to generate the markdown content
fn generate_markdown<W: Write>(
    writer: &mut W,
    root_dir: &Path,
    hidden: bool,
    output_path_for_filter: Option<PathBuf>, // Pass canonicalized path if writing to file
) -> io::Result<()> {
    // Create a HashSet for efficient lock file checking
    let lock_file_set: HashSet<&str> = LOCK_FILES.iter().cloned().collect();

    // Use WalkBuilder to respect .gitignore, .ignore, etc.
    let walker = WalkBuilder::new(&root_dir)
        .hidden(!hidden) // Use the passed 'hidden' flag
        .parents(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        // Add a filter predicate to explicitly ignore the output file and lock files
        .filter_entry(move |entry| {
            // --- Filter 1: Output File ---
            if let Some(output_path_to_check) = &output_path_for_filter {
                // Attempt canonicalization for comparison, proceed if it fails
                if let Ok(entry_path_canonical) = fs::canonicalize(entry.path()) {
                    if entry_path_canonical == *output_path_to_check {
                        return false; // Skip output file
                    }
                }
                // If canonicalization fails, don't skip based on this check
            }

            // --- Filter 2: Lock Files ---
            // Check only if it's a file to avoid matching directory names
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                // Check if the filename exists in our lock file set
                if let Some(file_name) = entry.file_name().to_str() {
                    if lock_file_set.contains(file_name) {
                        return false; // Skip lock file
                    }
                }
            }

            // --- Default: Include ---
            // If neither filter matched, include the entry
            true
        })
        .build();

    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if path == root_dir {
                    continue;
                } // Skip root dir itself
                if path.is_file() {
                    if let Ok(relative_path) = path.strip_prefix(&root_dir) {
                        if relative_path.as_os_str().is_empty() {
                            continue;
                        }
                        process_file(writer, relative_path, path)?;
                    } else {
                        eprintln!(
                            "Warning: Could not get relative path for {}",
                            path.display()
                        );
                    }
                }
            }
            Err(err) => eprintln!("Error accessing entry: {}", err),
        }
    }

    Ok(())
}

fn process_file<W: Write>(
    writer: &mut W,
    relative_path: &Path,
    full_path: &Path,
) -> io::Result<()> {
    writeln!(writer, "\n## `{}`\n", relative_path.display())?;

    match fs::read(full_path) {
        Ok(content) => {
            let content_type = content_inspector::inspect(&content);

            if content_type == ContentType::BINARY {
                writeln!(writer, "```\n(Binary file, content omitted)\n```")?;
            } else {
                let content_str = String::from_utf8_lossy(&content);
                let lang = get_language_tag(relative_path);
                writeln!(writer, "```{}", lang)?;
                for line in content_str.lines() {
                    writeln!(writer, "{}", line)?;
                }
                writeln!(writer, "```")?;
            }
        }
        Err(e) => {
            writeln!(writer, "```\n(Error reading file: {})\n```", e)?;
            eprintln!(
                "Warning: Failed to read file {}: {}",
                full_path.display(),
                e
            );
        }
    }
    Ok(())
}

fn get_language_tag(path: &Path) -> &str {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "py" | "pyw" => "python",
            "js" | "mjs" | "cjs" => "javascript",
            "ts" | "mts" | "cts" => "typescript",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cxx" | "hxx" | "cc" | "hh" => "cpp",
            "cs" => "csharp",
            "go" => "go",
            "php" => "php",
            "rb" => "ruby",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "scala" => "scala",
            "pl" => "perl",
            "sh" | "bash" | "zsh" => "bash",
            "ps1" => "powershell",
            "html" | "htm" => "html",
            "css" => "css",
            "scss" | "sass" => "scss",
            "less" => "less",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "md" | "markdown" => "markdown",
            "sql" => "sql",
            "xml" => "xml",
            "dockerfile" | "containerfile" => "dockerfile",
            "nix" => "nix",
            "lua" => "lua",
            "r" => "r",
            "dart" => "dart",
            "ex" | "exs" => "elixir",
            "erl" | "hrl" => "erlang",
            "hs" => "haskell",
            "clj" | "cljs" | "cljc" | "edn" => "clojure",
            "groovy" | "gradle" => "groovy",
            "tf" => "terraform",
            "vue" => "vue",
            "svelte" => "svelte",
            "tex" => "latex",
            "zig" => "zig",
            _ => "", // Default to no language tag
        })
        .unwrap_or("") // Handle cases with no extension
}
