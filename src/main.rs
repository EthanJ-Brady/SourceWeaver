// src/main.rs
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
    /// The path to the output markdown file.
    #[arg(short, long, default_value = "codebase.md")]
    output: PathBuf,

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

    println!("Scanning directory: {}", root_dir.display());
    println!("Outputting to: {}", args.output.display());

    // Ensure parent directory for output exists before canonicalizing
    if let Some(parent) = args.output.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Create the output file *first* so we can canonicalize its path
    // Use a block to ensure the file handle is closed before canonicalization
    {
        let _ = File::create(&args.output)?;
    }

    // Get the canonical (absolute, symlinks resolved) path of the output file
    let canonical_output_path = match fs::canonicalize(&args.output) {
        Ok(path) => Some(path),
        Err(e) => {
            eprintln!(
                "Warning: Could not canonicalize output path {}: {}. It might be included if inside the scanned directory.",
                args.output.display(),
                e
            );
            None
        }
    };

    // Create a HashSet for efficient lock file checking
    let lock_file_set: HashSet<&str> = LOCK_FILES.iter().cloned().collect();

    let output_file_handle = File::create(&args.output)?; // Re-open for writing
    let mut writer = BufWriter::new(output_file_handle);

    // Use WalkBuilder to respect .gitignore, .ignore, etc.
    let walker = WalkBuilder::new(&root_dir)
        .hidden(!args.hidden)
        .parents(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        // Add a filter predicate to explicitly ignore the output file and lock files
        .filter_entry(move |entry| {
            // --- Filter 1: Output File ---
            if let Some(output_path_to_check) = &canonical_output_path {
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
                        process_file(&mut writer, relative_path, path)?;
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

    println!("Successfully wrote codebase to {}", args.output.display());
    Ok(())
}

// process_file and get_language_tag functions remain unchanged
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
