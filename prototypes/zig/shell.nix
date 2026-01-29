{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    zig
  ];

  shellHook = ''
    echo "Zig development environment loaded"
    echo "Zig version: $(zig version)"
  '';
}
