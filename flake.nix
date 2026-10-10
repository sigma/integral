{
  description = "Integral — Integra-7 control surface";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
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
            "clippy"
          ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.wasm-pack
            pkgs.wasm-bindgen-cli
            pkgs.nodejs
            pkgs.typescript
            pkgs.cargo-watch
            pkgs.just
            pkgs.nixfmt
            pkgs.pkg-config
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            pkgs.alsa-lib
            pkgs.libjack2
            # baseview (via the Vizia editor): x11, xcb and xcb-util crates
            pkgs.libGL
            pkgs.libx11
            pkgs.libxcursor
            pkgs.libxcb
            pkgs.libxcb-wm
          ];

          shellHook = ''
            echo "Integral dev shell — Rust $(rustc --version | cut -d' ' -f2)"
          '';
        };
      }
    );
}
