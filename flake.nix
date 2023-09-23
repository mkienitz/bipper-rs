{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {
    self,
    crane,
    flake-utils,
    nixpkgs,
    ...
  }:
    {
      nixosModules.bipper = import ./nix/module.nix inputs;
      nixosModules.default = self.nixosModules.bipper;
      overlays.default = final: prev: {
        bipper = self.packages.${prev.stdenv.hostPlatform.system}.bipper;
      };
    }
    // flake-utils.lib.eachDefaultSystem
    (
      system: let
        craneLib = crane.lib.${system};
        pkgs = import nixpkgs {inherit system;};
        migrationsFilter = path: _type: builtins.match ".*/migrations/.*$" path != null;
        cargoFilter = craneLib.filterCargoSources;
        srcFilter = path: type: builtins.any (f: f path type) [cargoFilter migrationsFilter];
        src = nixpkgs.lib.cleanSourceWith {
          src = ./.;
          filter = srcFilter;
        };

        bipper = craneLib.buildPackage {
          inherit src;
          # src = craneLib.cleanCargoSource (craneLib.path ./.);
          buildInputs =
            [
              # Add additional build inputs here
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.libiconv
            ];
        };
      in {
        checks = {
          inherit bipper;
        };

        nixosTests.bipper = import ./nix/tests.nix {inherit pkgs self;};

        packages.default = bipper;
        packages.bipper = bipper;
        formatter = pkgs.alejandra;
        devShells.default = craneLib.devShell {
          inputsFrom = [bipper];
          buildInputs = [
            pkgs.SDL2
          ];
          packages = with pkgs; [
            alejandra
            cargo
            cargo-watch
            clippy
            deadnix
            nil
            rust-analyzer
            rustc
            rustfmt
          ];
        };
      }
    );
}
