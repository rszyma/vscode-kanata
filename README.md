# vscode-kanata

A VS Code extension that adds language support for [kanata](https://github.com/jtroo/kanata) configuration files .

### Features

A list of features is available [here](./vscode/README.md#features)

### Installing from source

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
cd vscode &&
make package
# or alternatively build in release mode:
# make CARGO_FLAGS=--release package
```
2. To install:
    - Right click on `kanata.vsix` file in VS Code file explorer and "Install Extension VSIX"...
    - ...or run `code --install-extension kanata.vsix` and restart VS Code.

### Credits

- https://github.com/jtroo/kanata/ - provides kanata-parser crate
- https://github.com/osohq/oso - used this as vscode extension template (with a lot of things removed)
- https://github.com/canadaduane/vscode-kmonad - syntax highlighting config
- https://github.com/jtroo/kanata/blob/main/assets/kanata-icon.svg - kanata icon
