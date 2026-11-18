{
  description = "Native Linux hardware control utility for Lenovo Legion laptops (Pro 5 16ADR10)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system:
        let pkgs = import nixpkgs { inherit system; };
        in f system pkgs
      );
    in
    {
      packages = forAllSystems (system: pkgs: {
        legion-cli = pkgs.callPackage ./nix/package.nix { };
        default = self.packages.${system}.legion-cli;
      });

      apps = forAllSystems (system: pkgs: {
        legion-cli = {
          type = "app";
          program = "${self.packages.${system}.legion-cli}/bin/legion-cli";
        };
        legionctl = {
          type = "app";
          program = "${self.packages.${system}.legion-cli}/bin/legionctl";
        };
        default = self.apps.${system}.legion-cli;
      });

      devShells = forAllSystems (system: pkgs: {
        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
          ];
        };
      });

      nixosModules = {
        default = import ./nix/nixos-module.nix;
        lenovo-legion = self.nixosModules.default;
      };

      overlays.default = final: prev: {
        legion-cli = final.callPackage ./nix/package.nix { };
      };
    };
}
