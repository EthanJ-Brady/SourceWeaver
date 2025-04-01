{
  description = "A tool to bundle a codebase into a single Markdown file respecting .gitignore";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      # Use the latest stable Rust toolchain from the overlay
      rustToolchain = pkgs.rust-bin.stable.latest.default;

      # Build the Rust package
      sourceweaver-pkg = pkgs.rustPlatform.buildRustPackage {
        pname = "sourceweaver";
        version = "0.2.0"; # Match Cargo.toml

        src = self; # Use the flake's source tree

        cargoLock.lockFile = ./Cargo.lock;

        # Ensure the toolchain is available
        buildInputs = [rustToolchain];
        # Provide cargo and rustc in the build environment
        nativeBuildInputs = [pkgs.cargo pkgs.rustc];
      };
    in {
      # Default package accessible via `nix build .#`
      packages.default = sourceweaver-pkg;

      # Allow running directly using `nix run .# -- <args>`
      apps.default = flake-utils.lib.mkApp {
        drv = sourceweaver-pkg;
      };

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          rustToolchain
          rust-analyzer
          pkg-config
          cargo-watch
          clippy
        ];
      };
    });
}
