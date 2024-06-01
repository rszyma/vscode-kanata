{
  # testing flake: nix develop --unset PATH

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay/stable";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        target = "wasm32-unknown-unknown";
        targetUpperSnake = pkgs.lib.toUpper (builtins.replaceStrings [ "-" ] [ "_" ] target);
        toolchainOverride = {
          extensions = [ "rust-src" ];
          targets = [ target ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
          "CARGO_TARGET_${targetUpperSnake}_LINKER" = "${pkgs.lld_18}/bin/lld";
          RUSTFLAGS = nixpkgs.lib.strings.concatStringsSep " " [
            # "-C link-arg=-fuse-ld=${pkgs.mold}"
            # "-C link-arg=--ld-path=${pkgs.mold}"
            # "-Zlinker-features=-lld"
          ];
          nativeBuildInputs = with pkgs; [
            # (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override toolchainOverride))
            (rust-bin.stable.latest.default.override toolchainOverride)
            # (rust-bin.stable."1.73.0".default.override toolchainOverride)
            just
            git
            yarn
            wasm-pack
          ];
        };
      }
    );
}
