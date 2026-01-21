{
  description = "DevShell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = "1.92.0";
        rust = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [ "rust-analyzer" "rust-src" ];
        };
        commonPackages = with pkgs; [
          cargo-edit
          cargo-nextest
          cargo-outdated
          cargo-watch
          fd
          ripgrep
          rust
        ];
        darwinPackages = with pkgs; [
          libiconv
        ];
        linuxPackages = with pkgs; [
          glibc
          glibcLocales
        ];
        platformPackages =
          if pkgs.stdenv.isDarwin then
            commonPackages ++ darwinPackages
          else
            commonPackages ++ linuxPackages;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = platformPackages;

          shellHook = ''
            echo ""
            echo "  Rust Development Shell"
            echo "  ───────────────────────────"
            echo "  Rust:        $(rustc --version | cut -d' ' -f2)"
            echo ""
          '' + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            export LOCALE_ARCHIVE="${pkgs.glibcLocales}/lib/locale/locale-archive"
          '';
        };
      });
}
