{ pkgs }:

let
  version = "0.0.142";
in
if pkgs.stdenv.isLinux then
  let
    arch = if pkgs.stdenv.hostPlatform.isAarch64 then "aarch64" else "x86_64";
    sha256 =
      if pkgs.stdenv.hostPlatform.isAarch64 then
        "6d6239e28ac67067b15c92ede1a103da20cd7bae9cd4da716c9109a35a8d9ad4"
      else
        "e4ba5bccbfb511494033b2d7dceba09a2ee72807d3426112426521de46a4233b";
  in
  pkgs.stdenv.mkDerivation {
    pname = "nu-lint";
    inherit version;
    src = pkgs.fetchurl {
      url =
        "https://codeberg.org/wvhulle/nu-lint/releases/download/v${version}/nu-lint-${arch}-unknown-linux-musl.tar.gz";
      inherit sha256;
    };
    nativeBuildInputs = [ pkgs.gnutar ];
    phases = [ "unpackPhase" "installPhase" ];
    unpackPhase = ''
      tar -xzf $src
    '';
    installPhase = ''
      mkdir -p $out/bin
      cp nu-lint $out/bin/
      chmod +x $out/bin/nu-lint
    '';
  }
else
  pkgs.rustPlatform.buildRustPackage {
    pname = "nu-lint";
    inherit version;
    src = pkgs.fetchgit {
      url = "https://codeberg.org/wvhulle/nu-lint.git";
      rev = "v${version}";
      sha256 = "sha256-Hdm1JtChg5dSnPKO8q2EVVGUL4978ZK97trvi9J1rIc=";
    };
    cargoHash = "sha256-yqF+jDaQ7DwLRUZvvwsTRDmeB70Qsw0ywO4RzcGrXRo=";
  }
