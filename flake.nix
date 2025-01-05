{
  inputs = {
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    nci = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      imports = [
        inputs.devshell.flakeModule
        inputs.flake-parts.flakeModules.easyOverlay
        inputs.nci.flakeModule
        inputs.pre-commit-hooks.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      perSystem =
        {
          pkgs,
          config,
          ...
        }:
        let
          crateName = "bipper";
          projectName = crateName;
          crateOutput = config.nci.outputs.${crateName};
        in
        {
          devshells.default = {
            packages = [
              pkgs.nil
              pkgs.rust-analyzer
            ];
            devshell.startup.pre-commit.text = config.pre-commit.installationScript;
          };

          pre-commit.settings.hooks.treefmt.enable = true;

          treefmt = {
            projectRootFile = "flake.nix";
            programs = {
              deadnix.enable = true;
              statix.enable = true;
              nixfmt.enable = true;
              rustfmt.enable = true;
            };
          };

          nci = {
            projects.${projectName} = {
              path = ./.;
              numtideDevshell = "default";
            };
            crates.${crateName} = { };
            # migrationsFilter = path: _type: builtins.match ".*/migrations/.*$" path != null;
            # srcFilter = path: type: builtins.any (f: f path type) [cargoFilter migrationsFilter];
          };

          packages.default = crateOutput.packages.release;

          overlayAttrs.coffee-labeler = config.packages.default;
        };

      flake =
        {
          config,
          pkgs,
          self,
          ...
        }:
        {
          nixosModules = {
            bipper = import ./nix/module.nix inputs;
            default = config.nixosModules.bipper;
          };
          nixosTests.bipper = import ./nix/tests.nix { inherit pkgs self; };
        };
    };
}
