{
  description = "Orbit - Autonomous Software Engineering Server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          openssl
          git
          curl
          sqlite
          postgresql
        ];

        buildInputs = with pkgs; [
          openssl
          pkg-config
          sqlite
          postgresql
        ];

        orbitCode = pkgs.rustPlatform.buildRustPackage {
          pname = "orbit-code";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = nativeBuildInputs;
          buildInputs = buildInputs;

          doCheck = true;
          checkInputs = with pkgs; [ postgresql sqlite ];

          meta = with pkgs.lib; {
            description = "Autonomous software engineering server";
            homepage = "https://github.com/frontal-labs/frontal-orbit";
            license = licenses.mit;
            platforms = platforms.linux ++ platforms.darwin;
          };
        };
      in
      {
        packages = {
          default = orbitCode;
          orbit-code = orbitCode;
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          CARGO_TERM_COLOR = "always";
          DATABASE_URL = "sqlite:///tmp/orbit_dev.db";
        };

        apps = {
          orbit = {
            type = "app";
            program = "${orbitCode}/bin/orbit";
          };
        };
      });
}