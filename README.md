# Source Weaver

Source Weaver is a command-line tool that scans a codebase directory, respects `.gitignore` rules, and bundles all non-ignored source files into a single Markdown file. Each file's content is placed within a fenced code block, tagged with its language (if recognized), and preceded by its relative path.

## Features

- **Codebase Bundling:** Consolidates an entire project's text files into one Markdown document.
- **`.gitignore` Aware:** Automatically respects rules found in `.gitignore`, `.ignore`, `.git/info/exclude`, and global gitignore files. Also respects ignore rules in parent directories.
- **Language Detection:** Adds language tags (e.g., `rust`, `python`, `javascript`) to Markdown code blocks based on file extensions for syntax highlighting.
- **Binary File Handling:** Detects binary files and includes a placeholder instead of attempting to render their content.
- **Hidden File Control:** Ignores hidden files/directories (starting with `.`) by default, but can be configured to include them.
- **Cross-Platform:** Built with Rust, runs on Linux, macOS, and Windows.
- **Nix Flake:** Provides a Nix flake for reproducible builds and development environments.

## Installation

### Option 1: Using Cargo (Requires Rust toolchain)

1.  **Install Rust:** If you haven't already, install Rust via [rustup](https://rustup.rs/).
2.  **Clone the repository (Optional):**
    ```bash
    git clone https://github.com/EthanJ-Brady/SourceWeaver
    cd sourceweaver
    ```
3.  **Install:**
    - From a local clone:
      ```bash
      cargo install --path .
      ```
    - Directly from a Git repository:
      ```bash
      cargo install --git https://github.com/EthanJ-Brady/SourceWeaver sourceweaver
      ```

After installation, the `sourceweaver` binary should be available in your Cargo bin path (`~/.cargo/bin/` by default).

### Option 2: Using Nix (Requires Nix with Flakes enabled)

1.  **Enable Flakes:** Ensure Nix flakes are enabled in your Nix configuration.
2.  **Build and Run Directly:**
    - From a local clone:
      ```bash
      # Navigate to the sourceweaver directory
      nix run .# -- --help
      ```
    - Directly from a Git repository:
      ```bash
      nix run github:EthanJ-Brady/SourceWeaver -- --help
      ```
3.  **Install to Profile:**
    - From a local clone:
      ```bash
      # Navigate to the sourceweaver directory
      nix profile install .#
      ```
    - Directly from a Git repository:
      ```bash
      nix profile install github:EthanJ-Brady/SourceWeaver
      ```

This makes the `sourceweaver` command available in your user profile.

## Usage

Run `sourceweaver` from within the root directory of the project you want to process.

```bash
# Basic usage: Process current directory, output to codebase.md
sourceweaver

# Specify output file name
sourceweaver -o my_project_bundle.md
# or
sourceweaver --output my_project_bundle.md

# Specify a different root directory to process
sourceweaver --root /path/to/another/project -o another_project.md

# Include hidden files (e.g., .envrc, .config files if not ignored)
sourceweaver --hidden -o project_with_hidden.md

# Get help
sourceweaver --help
```

### Arguments

- `-o, --output <FILE>`
  Sets the output Markdown file path.
  (Default: `codebase.md`)

- `-r, --root <DIR>`
  Sets the root directory of the codebase to scan.
  (Default: current working directory)

- `--hidden`
  Include hidden files and directories (those starting with `.`) that are not otherwise ignored by gitignore rules.

- `-h, --help`
  Print help information.

- `-V, --version`
  Print version information.
