{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      perSystem =
        {
          system,
          ...
        }:
        let
          overlays = [ inputs.rust-overlay.overlays.default ];
          pkgs = import inputs.nixpkgs {
            inherit system overlays;
          };
          rustToolchain = pkgs.rust-bin.stable."1.88.0".default;

          rust-toolchain = pkgs.symlinkJoin {
            name = "rust-toolchain";
            paths = [
              rustToolchain
              pkgs.cargo-watch
              pkgs.rust-analyzer
              pkgs.cargo-dist
              pkgs.cargo-tarpaulin
              pkgs.cargo-insta
              pkgs.cargo-machete
              pkgs.cargo-edit
              pkgs.cargo-flamegraph
            ];
          };
        in
        {
          devShells.default = pkgs.mkShell {
            RUST_BACKTRACE = "full";
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            buildInputs = with pkgs; [
              openssl
            ];
            packages = with pkgs; [
              # Nix
              nil
              alejandra

              # Typst
              typst
              tinymist
              typstyle

              rust-toolchain
            ];
          };
        };
    };
}
