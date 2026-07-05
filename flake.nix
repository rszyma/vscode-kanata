{
  # testing flake: nix develop --unset PATH

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay/stable";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        targets = [
          "wasm32-unknown-unknown"
          "x86_64-unknown-linux-gnu"
        ];
        toolchainOverride = {
          extensions = [ "rust-src" ];
          inherit targets;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            # (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override toolchainOverride))
            (rust-bin.stable.latest.default.override toolchainOverride)
            # (rust-bin.stable."1.73.0".default.override toolchainOverride)
            just
            yarn
            wasm-pack
            vsce
            (pkgs.writeShellScriptBin "ovsx" "${pkgs.nodejs_24}/bin/npx ovsx $@")
          ];
        };
      }
    );
}
