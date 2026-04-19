{
  description = "Linter for detecting AI generated slop";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      nu-lint,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rustToolchain = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
        rustVersion = rustToolchain.toolchain.channel;
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        nu-lint-package = nu-lint.packages.${system}.default;
        revision = self.rev or "dirty";
        sandboxPath = pkgs.lib.makeBinPath [
          rust
          pkgs.bash
          pkgs.bun
          pkgs.cargo-edit
          pkgs.cargo-nextest
          pkgs.cargo-outdated
          pkgs.cargo-watch
          pkgs.coreutils
          pkgs.docker-client
          pkgs.fd
          pkgs.findutils
          pkgs.git
          pkgs.gnugrep
          pkgs.just
          pkgs.nushell
          pkgs.ripgrep
          pkgs.taplo
          pkgs.zip
          pkgs.stdenv.cc
        ];
        sandbox = pkgs.writeShellScriptBin "sandbox" ''
          SELF_DIR="$(dirname "$(readlink -f "$0")")"
          SANDBOX_CMD="''${@:-$SHELL}"
          ARGS=(
            --unshare-user
            --unshare-ipc
            --unshare-uts
            --unshare-cgroup
            --share-net
            --die-with-parent
            --ro-bind /nix /nix
            --ro-bind /etc/resolv.conf /etc/resolv.conf
            --ro-bind /etc/ssl /etc/ssl
            --ro-bind-try /etc/static /etc/static
            --ro-bind-try /etc/nix /etc/nix
            --ro-bind-try /run/current-system/sw /run/current-system/sw
            --symlink ${pkgs.coreutils}/bin/env /usr/bin/env
            --symlink ${pkgs.bash}/bin/bash /bin/sh
            --tmpfs "$HOME"
            --bind "$(pwd)" "$(pwd)"
            --bind /tmp /tmp
            --ro-bind "$(git rev-parse --git-common-dir)" "$(git rev-parse --git-common-dir)"
            --ro-bind-try "$HOME/.gitconfig" "$HOME/.gitconfig"
            --ro-bind-try "$HOME/.ssh/known_hosts" "$HOME/.ssh/known_hosts"
            --setenv PATH "${sandboxPath}:$SELF_DIR"
            --setenv HOME "$HOME"
            --setenv REVISION "''${REVISION:-dev}"
            --dev /dev
            --proc /proc
          )
          ARGS+=(
            --bind-try "$HOME/.cargo" "$HOME/.cargo"
            --bind-try "$HOME/.bun" "$HOME/.bun"
          )
          exec ${pkgs.bubblewrap}/bin/bwrap "''${ARGS[@]}" -- $SANDBOX_CMD
        '';
        commonPackages = with pkgs; [
          deadnix
          fd
          just
          nixfmt
          nu-lint-package
          nushell
          ripgrep
          sandbox
          statix
          taplo
          woodpecker-cli
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
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          inherit (cargoToml.package) version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          REVISION = revision;
          CARGO_HOME = "$TMPDIR";
          doCheck = false;
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = platformPackages;

          shellHook = ''
            echo ""
            echo "  ${cargoToml.package.name} Development Shell"
            echo "  ───────────────────────────"
            echo "  Rust:        ${rustVersion} (sandbox)"
            echo "  Bun:         ${pkgs.bun.version} (sandbox)"
            echo "  Woodpecker:  ${pkgs.woodpecker-cli.version} (sandbox)"
            echo ""
          ''
          + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            export LOCALE_ARCHIVE="${pkgs.glibcLocales}/lib/locale/locale-archive"
          '';
        };
      }
    );
}
