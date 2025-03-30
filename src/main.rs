// src/main.rs
use clap::Parser;
use content_inspector::ContentType;
use ignore::WalkBuilder;
use std::{
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

fn main() -> io::Result<()> {
    let args = Args::parse();

    let root_dir = args
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    println!("Scanning directory: {}", root_dir.display());
    println!("Outputting to: {}", args.output.display());

    // --- Bug Fix Start ---
    // Ensure parent directory for output exists before canonicalizing
    if let Some(parent) = args.output.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Create the output file *first* so we can canonicalize its path
    let output_file = File::create(&args.output)?;

    // Get the canonical (absolute, symlinks resolved) path of the output file
    // This is crucial for accurate comparison during the walk.
    let canonical_output_path = match fs::canonicalize(&args.output) {
        Ok(path) => Some(path),
        Err(e) => {
            // If we can't canonicalize (e.g., permissions, weird paths),
            // warn the user and proceed without filtering. The file *might*
            // still be included if it's inside the scanned directory.
            eprintln!(
                "Warning: Could not canonicalize output path {}: {}. It might be included if inside the scanned directory.",
                args.output.display(),
                e
            );
            None // Proceed without filtering
        }
    };
    // --- Bug Fix End ---

    let mut writer = BufWriter::new(output_file);

    // Use WalkBuilder to respect .gitignore, .ignore, etc.
    let walker = WalkBuilder::new(&root_dir)
        .hidden(!args.hidden)
        .parents(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        // --- Bug Fix Start ---
        // Add a filter predicate to explicitly ignore the output file
        .filter_entry(move |entry| {
            // If we determined the canonical output path earlier, use it for filtering
            if let Some(output_path_to_check) = &canonical_output_path {
                // Try to canonicalize the entry's path for reliable comparison
                // If canonicalization fails for the entry (e.g., broken symlink),
                // we default to NOT filtering it (return true).
                match fs::canonicalize(entry.path()) {
                    Ok(entry_path_canonical) => {
                        // Filter out (return false) if the entry's canonical path
                        // matches the output file's canonical path.
                        entry_path_canonical != *output_path_to_check
                    }
                    Err(_) => true, // Don't filter if canonicalization fails
                }
            } else {
                // If we couldn't get the canonical output path initially,
                // don't filter any entries based on it.
                true
            }
        })
        // --- Bug Fix End ---
        .build(); // Don't use build_parallel() to maintain order

    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path();
                // Skip the root directory itself if it happens to be yielded
                if path == root_dir {
                    continue;
                }
                if path.is_file() {
                    if let Ok(relative_path) = path.strip_prefix(&root_dir) {
                        // Skip empty relative paths (shouldn't happen often)
                        if relative_path.as_os_str().is_empty() {
                            continue;
                        }
                        process_file(&mut writer, relative_path, path)?;
                    } else {
                        eprintln!(
                            "Warning: Could not get relative path for {}",
                            path.display()
                        );
                        // Avoid processing if we can't get a relative path
                    }
                }
            }
            Err(err) => eprintln!("Error accessing entry: {}", err),
        }
    }

    println!("Successfully wrote codebase to {}", args.output.display());
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
                // Ensure consistent line endings (Unix-style) in output
                for line in content_str.lines() {
                    writeln!(writer, "{}", line)?;
                }
                // writeln!(writer, "{}", content_str.trim_end())?; // Alternative: trim trailing newline
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

// Basic language detection based on file extension (no changes needed here)
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
