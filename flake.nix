{
  inputs = {
    # use release 23.05 branch of the GitHub repository as input, this is the most common input format
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.05";
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {
    nixpkgs,
    flake-utils,
    gitignore,
    ...
  }:
    flake-utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
		    darwin.apple_sdk.frameworks.SystemConfiguration
			libiconv
          ];
          packages = with pkgs; [
            sqlite
            rust-analyzer
			diesel-cli
            alejandra
          ];
        };
      }
    );
}
