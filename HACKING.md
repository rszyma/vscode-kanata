## Building from source

Requirements:
- Latest stable version of Rust with `cargo` available on your system PATH.
- `wasm-pack` 0.12.1+ installed and available on your system PATH.
- VS Code 1.80.0+.
- yarn

1. Run commands:
```bash
git submodule init
git submodule update
make package
# or alternatively build in release mode:
# make CARGO_FLAGS=--release package
```
1. To install:
    - Right click on `kanata.vsix` file in VS Code file explorer and "Install Extension VSIX"...
    - ...or run `code --install-extension kanata.vsix` and restart VS Code.

Alternatively build + install can be done with `just install`, if you have installed [just](https://github.com/casey/just).
See [justfile](./justfile) for other just commands.

## Debugging the extension

All logs from the extension can be viewed in `Output > Kanata Configuration Language`.

Logging from Rust can be done using `log!` macro. Grep the code for examples.

Logging from JS/TS can be done using `console.log`.

Logging of LSP protocol communications can be enabled by adding this to your `settings.json` (vscode settings):

```
"vscode-kanata.trace.server": "verbose"
```

The workflow I use for development:
1. Change something in vscode-kanata
2. Run `just install`
3. In the other vscode window, where I have some configs to test, run vscode command `Developer: Reload Window` to reload all extensions.
4. Test manually, and repeat.

## Bumping the version of kanata

This project directly embeds kanata's parser during compilation.
It's located in project root in `/kanata`.
Any updates to kanata must be manually checked out.
The following command bumps version of kanata to the latest commit on [kanata main branch](https://github.com/jtroo/kanata/tree/main):

```bash
git submodule update --remote
```
