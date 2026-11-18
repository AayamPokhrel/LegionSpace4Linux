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
        legionspace = pkgs.callPackage ./nix/package.nix { };
        legion-cli = self.packages.${system}.legionspace;
        default = self.packages.${system}.legionspace;
      });

      apps = forAllSystems (system: pkgs: {
        legionspace = {
          type = "app";
          program = "${self.packages.${system}.legionspace}/bin/legionspace";
        };
        legionctl = {
          type = "app";
          program = "${self.packages.${system}.legionspace}/bin/legionctl";
        };
        legion-cli = {
          type = "app";
          program = "${self.packages.${system}.legionspace}/bin/legion-cli";
        };
        default = self.apps.${system}.legionspace;
      });

      devShells = forAllSystems (system: pkgs: {
        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            pkg-config
            wayland
            libxkbcommon
            libGL
          ];
        };
      });

      nixosModules = {
        default = import ./nix/nixos-module.nix;
        lenovo-legion = self.nixosModules.default;
      };

      overlays.default = final: prev: {
        legionspace = final.callPackage ./nix/package.nix { };
        legion-cli = final.callPackage ./nix/package.nix { };
      };
    };
}
