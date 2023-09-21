{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {
    crane,
    flake-utils,
    nixpkgs,
    ...
  }:
    flake-utils.lib.eachDefaultSystem
    (
      system: let
        craneLib = crane.lib.${system};
        pkgs = import nixpkgs {inherit system;};

        bipper = craneLib.buildPackage {
          src = craneLib.cleanCargoSource (craneLib.path ./.);
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

        packages.default = bipper;
        packages.bipper = bipper;
        formatter = pkgs.alejandra;
        devShells.default = craneLib.devShell {
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

        # devShells.default = pkgs.mkShell {
        #   buildInputs = with pkgs; [
        #     darwin.apple_sdk.frameworks.SystemConfiguration
        #     libiconv
        #     SDL2
        #   ];
        #   packages = with pkgs; [
        #     alejandra
        #     cargo
        #     cargo-watch
        #     clippy
        #     deadnix
        #     nil
        #     rust-analyzer
        #     rustc
        #     sqlite
        #   ];
        # };
      }
    );
}
