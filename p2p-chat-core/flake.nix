{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
      ...
    }@inputs:
    let
      eachSystem =
        f:
        nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed (system: f nixpkgs.legacyPackages.${system});
    in
    {
      # packages = eachSystem (pkgs: {
      # });

      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell (
          with pkgs;
          {
            buildInputs = [
              sqlite
              bacon
              cargo
              clippy
              git
              gcc
              rustc
              rust-analyzer
              openssl
              pkg-config
            ];
          }
        );
      });

    };
}
