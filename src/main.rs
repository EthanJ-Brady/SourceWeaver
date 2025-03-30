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

    let output_file = File::create(&args.output)?;
    let mut writer = BufWriter::new(output_file);

    // Use WalkBuilder to respect .gitignore, .ignore, etc.
    // By default, it also ignores hidden files unless overridden.
    let walker = WalkBuilder::new(&root_dir)
        .hidden(!args.hidden) // Ignore hidden files unless --hidden is passed
        .parents(true) // Process parent ignore files (.gitignore in parent dirs)
        .git_ignore(true) // Use .gitignore
        .git_global(true) // Use global gitignore
        .git_exclude(true) // Use .git/info/exclude
        .ignore(true) // Use .ignore files
        .build(); // Don't use build_parallel() to maintain order

    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    // Calculate relative path from the root_dir
                    if let Ok(relative_path) = path.strip_prefix(&root_dir) {
                        process_file(&mut writer, relative_path, path)?;
                    } else {
                        // This might happen if the path is somehow outside the root
                        // or if root_dir is "." and path is absolute (less likely with WalkBuilder)
                        eprintln!(
                            "Warning: Could not get relative path for {}",
                            path.display()
                        );
                        process_file(&mut writer, path, path)?; // Use full path as fallback
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
    // Write the header with the relative file path
    writeln!(writer, "\n## `{}`\n", relative_path.display())?;

    // Attempt to read the file content
    match fs::read(full_path) {
        Ok(content) => {
            // Check if the content looks like text (specifically UTF-8)
            let content_type = content_inspector::inspect(&content);

            if content_type == ContentType::BINARY {
                writeln!(writer, "```\n(Binary file, content omitted)\n```")?;
            } else {
                // Attempt to convert to UTF-8, replacing invalid sequences
                let content_str = String::from_utf8_lossy(&content);
                let lang = get_language_tag(relative_path);
                writeln!(writer, "```{}", lang)?;
                writeln!(writer, "{}", content_str)?;
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

// Basic language detection based on file extension
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
