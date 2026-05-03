{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # nixpkgsにあるworker-buildが古い(0.7.3とか？)ので、自前でビルドします
        worker-build = pkgs.rustPlatform.buildRustPackage rec {
          pname = "worker-build";
          version = "0.8.1";

          src = pkgs.fetchCrate {
            inherit pname version;
            sha256 = "sha256-Df87FvodwFeI3UNdFSPqO2oDbnlEg0gWB8ZA1tk/NHo=";
          };

          cargoHash = "sha256-YGt+D7f5VUVnftc9TAeSjbh7a5YwXdFgm+XiaQwoiDA=";

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];

          doCheck = false;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            rustfmt

            openssl
            pkg-config

            wrangler
            worker-build
          ];
        };
      }
    );
}
