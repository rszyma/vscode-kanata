let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-unstable";
  pkgs = import nixpkgs { config = { }; overlays = [ ]; };
in

pkgs.mkShellNoCC {
  packages = with pkgs; [
    just
    nodejs_22
    yarn
    wasm-pack
  ];
}
