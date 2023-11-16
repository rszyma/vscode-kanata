
## Building from source

Requirements:
- Latest stable version of Rust with `cargo` available on your system PATH.
- `wasm-pack` 0.12.1+ installed and available on your system PATH.
- VS Code 1.80.0+.

1. Run commands:
```bash
git clone https://github.com/rszyma/vscode-kanata.git &&
cd vscode-kanata &&
git submodule init &&
git submodule update &&
make package
# or alternatively build in release mode:
# make CARGO_FLAGS=--release package
```
2. To install:
    - Right click on `kanata.vsix` file in VS Code file explorer and "Install Extension VSIX"...
    - ...or run `code --install-extension kanata.vsix` and restart VS Code.


## Bumping version of kanata

This project directly embeds kanata's parser during compilation. It's located in project root in `/kanata`. Any updates to kanata must be manually checked out. The following command bumps version of kanata to the latest commit on [kanata main branch](https://github.com/jtroo/kanata/tree/main):

```bash
git submodule update --remote
```