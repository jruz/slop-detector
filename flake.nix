{
  description = "DevShell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    nu-lint = {
      url = "git+https://codeberg.org/wvhulle/nu-lint?ref=refs/tags/v0.1.1";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      nu-lint,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
        rustVersion = rustToolchain.toolchain.channel;
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        nu-lint-package = nu-lint.packages.${system}.default;
        commonPackages = with pkgs; [
          bun
          cargo-edit
          cargo-nextest
          cargo-outdated
          cargo-watch
          deadnix
          fd
          just
          nixfmt
          nu-lint-package
          nushell
          ripgrep
          rust
          statix
          taplo
          woodpecker-cli
          zip
        ];
        darwinPackages = with pkgs; [
          libiconv
        ];
        linuxPackages = with pkgs; [
          glibc
          glibcLocales
        ];
        platformPackages =
          if pkgs.stdenv.isDarwin then commonPackages ++ darwinPackages else commonPackages ++ linuxPackages;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = platformPackages;

          shellHook = ''
            echo ""
            echo "  Rust Development Shell"
            echo "  ───────────────────────────"
            echo "  Rust:        ${rustVersion}"
            echo "  Bun:         ${pkgs.bun.version}"
            echo "  Nextest:     ${pkgs.cargo-nextest.version}"
            echo "  Woodpecker:  ${pkgs.woodpecker-cli.version}"
            echo ""
          ''
          + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            export LOCALE_ARCHIVE="${pkgs.glibcLocales}/lib/locale/locale-archive"
          '';
        };
      }
    );
}
