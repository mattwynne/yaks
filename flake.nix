{
  description = "Yaks - A non-linear TODO list for humans and robots";

  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, devenv, ... }@inputs:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.stdenv.mkDerivation {
            pname = "yx";
            version = "0.1.0";

            src = ./.;

            nativeBuildInputs = [ pkgs.zip ];

            buildPhase = ''
              zip -r yx.zip bin/yx completions/
            '';

            installPhase = ''
              mkdir -p $out
              cp yx.zip $out/
            '';

            meta = with pkgs.lib; {
              description = "A non-linear TODO list for humans and robots";
              homepage = "https://github.com/mattwynne/yaks";
              license = licenses.mit;
              platforms = platforms.unix;
            };
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = devenv.lib.mkShell {
            inherit inputs pkgs;
            modules = [
              ./devenv.nix
            ];
          };
        });
    };
}
