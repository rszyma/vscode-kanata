# vscode-kanata

A VS Code extension that adds language support for [kanata](https://github.com/jtroo/kanata) configuration files .

### Features

A list of features is available [here](./vscode/README.md#features)

### Installing from source

1. Run commands:
```
git clone https://github.com/rszyma/vscode-kanata.git
cd vscode-kanata/vscode
make package
```
2. Right click on `kanata.vsix` file VS Code file explorer and "Install Extension VSIX".

Also see [./vscode/DEVELOPMENT.md](./vscode/DEVELOPMENT.md)

### Credits

- https://github.com/jtroo/kanata/ - provides kanata-parser crate
- https://github.com/osohq/oso - used this as vscode extension template (with a lot of things removed)
- https://github.com/canadaduane/vscode-kmonad - syntax highlighting config
- https://github.com/jtroo/kanata/blob/main/assets/kanata-icon.svg - kanata icon
