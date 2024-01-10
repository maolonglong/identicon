{
  description = "identicon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [rust-overlay.overlays.default];
        pkgs = import nixpkgs {inherit system overlays;};
      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          ];
        };
      }
    );
}
