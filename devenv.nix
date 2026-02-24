{ pkgs, lib, ... }:

{
  packages = with pkgs; [
    adrgen
    shellspec
    git
    argc
    shellcheck
    prek
    pkg-config
    openssl
    zlib
    tmux
    vhs
    bashInteractive  # needed for completion smoke test (nix bash lacks readline)
  ] ++ lib.optionals pkgs.stdenv.isDarwin [
    libiconv
  ];

  languages.rust.enable = true;

  enterShell = ''
    echo "Yaks development environment loaded"

    # Build Rust binary if it doesn't exist
    if [ ! -f target/release/yx ]; then
      echo "Building Rust binary..."
      cargo build --release
    fi

    # Add target/release to PATH for tests
    export PATH="$PWD/target/release:$PATH"
  '';

  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';
}
