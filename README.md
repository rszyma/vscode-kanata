# vscode-kanata
[![GitHub Repo stars](https://img.shields.io/github/stars/rszyma/vscode-kanata?logo=github)](https://github.com/rszyma/vscode-kanata)
[![Visual Studio Marketplace Installs](https://img.shields.io/visual-studio-marketplace/i/rszyma.vscode-kanata?logo=visualstudiocode)](https://marketplace.visualstudio.com/items?itemName=rszyma.vscode-kanata)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/rszyma/vscode-kanata/rust.yml)
<!-- ![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/rszyma/vscode-kanata/publish.yml?label=nightly%20kanata%20version%20bump) -->
<!-- [![Visual Studio Marketplace Version (including pre-releases)](https://img.shields.io/visual-studio-marketplace/v/rszyma.vscode-kanata)](https://marketplace.visualstudio.com/items?itemName=rszyma.vscode-kanata) -->

A VS Code extension that adds language support for [kanata](https://github.com/jtroo/kanata) configuration files.

## Features

Kanata config files are detected by `.kbd` file extension.

### Syntax highlighting

keywords, action identifiers, alias handles etc.

<p><img src="assets/syntax-highlighting-showcase.png"/></p>

### Checking for config errors

Config will be parsed and validated, when saving document.

<p><img src="assets/config-parsing-showcase.gif"/></p>

Note: kanata parser is embedded directly in this extension, so it might happen
that this extension don't yet support the latest features of kanata. Version bumps of
kanata are usually done every release and indicated in [change log](/CHANGELOG.md).

### Support for including other files

If you use [`include`](https://github.com/jtroo/kanata/blob/main/docs/config.adoc#include-other-files)
configuration items in your kanata config, make sure to adjust the following settings:
- `vscode-kanata.includesAndWorkspaces`
- `vscode-kanata.mainConfigFile`

Important: Absolute paths in `include` blocks that point outside the opened workspace aren't supported.

Also, if you work with multiple main files, and find yourself switching `mainConfigFile` often,
there's a handy command palette entry:
- `Kanata: Set current file as main`

## Contributing

Contributions are welcome, feel free to open an issue or a PR.

### Bug reports

If you encounter a bug, please report it here: https://github.com/rszyma/vscode-kanata/issues

### Building

See [this document](CONTRIBUTING.md) for build instructions.

## Release notes

See the [change log](CHANGELOG.md).

## Credits

- https://github.com/jtroo/kanata/ - provides kanata-parser crate
- https://github.com/osohq/oso - used this as vscode extension template (with a lot of things removed)
- https://github.com/canadaduane/vscode-kmonad - syntax highlighting config
- https://github.com/jtroo/kanata/blob/main/assets/kanata-icon.svg - kanata icon
