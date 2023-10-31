{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {
    self,
    crane,
    flake-utils,
    devshell,
    nixpkgs,
    ...
  }:
    {
      nixosModules.bipper = import ./nix/module.nix inputs;
      nixosModules.default = self.nixosModules.bipper;
      overlays.default = final: prev: {
        bipper = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };
    }
    // flake-utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            devshell.overlays.default
          ];
        };

        # Crane build
        craneLib = crane.lib.${system};
        migrationsFilter = path: _type: builtins.match ".*/migrations/.*$" path != null;
        cargoFilter = craneLib.filterCargoSources;
        srcFilter = path: type: builtins.any (f: f path type) [cargoFilter migrationsFilter];
        src = nixpkgs.lib.cleanSourceWith {
          src = ./.;
          filter = srcFilter;
        };

        commonArgs = {
          inherit src;
          # src = craneLib.cleanCargoSource (craneLib.path ./.);
          buildInputs =
            [
              # Add additional build inputs here
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.darwin.apple_sdk.frameworks.CoreFoundation
              pkgs.libiconv
            ];
        };

        bipper = craneLib.buildPackage commonArgs;
      in {
        # nix flake check
        checks = {
          inherit bipper;
        };

        nixosTests.bipper = import ./nix/tests.nix {inherit pkgs self;};

        packages.default = bipper;

        packages.bipper-docker = pkgs.dockerTools.buildLayeredImage {
          name = "bipper";
          config.Cmd = ["${bipper}/bin/bipper"];
        };

        formatter = pkgs.alejandra;

        devShells.default = pkgs.devshell.mkShell {
          imports = [
            "${devshell}/extra/language/rust.nix"
            "${devshell}/extra/language/c.nix"
          ];
          packages = with pkgs;
            [
              rust-analyzer
            ]
            ++ commonArgs.buildInputs;
        };
      }
    );
}
