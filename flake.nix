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
          lib,
          ...
        }:
        let
          crateName = "bipper";
          projectName = crateName;
          crateOutput = config.nci.outputs.${crateName};
        in
        {
          devshells.default = {
            packages =
              [
                pkgs.nil
                pkgs.rust-analyzer
              ]
              ++ lib.optionals pkgs.stdenv.isDarwin [
                pkgs.libiconv
              ];
            env = [
              {
                name = "LIBRARY_PATH";
                eval = "$DEVSHELL_DIR/lib:/nix/store/lgcwrpj3yl6s2xsiwjxizbma9yi1p530-sqlite-3.47.0/lib";
              }
              {
                name = "BIPPER_ADDRESS";
                eval = "127.0.0.1";
              }
              {
                name = "BIPPER_PORT";
                eval = "3333";
              }
              {
                name = "BIPPER_DATABASE_PATH";
                eval = "db.sqlite";
              }
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
            crates.${crateName}.drvConfig.mkDerivation = {
              nativeBuildInputs = [
                # pkgs.sqlite.dev
                # pkgs.pkg-config
              ];
            };
          };

          packages.default = crateOutput.packages.release;

          overlayAttrs.bipper = config.packages.default;
        };

      flake =
        {
          config,
          ...
        }:
        {
          nixosModules = {
            bipper = import ./nix/module.nix inputs;
            default = config.nixosModules.bipper;
          };
        };
    };
}
