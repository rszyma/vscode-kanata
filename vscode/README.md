# Kanata configuration language support VS Code

Language support for .kbd kanata (https://github.com/jtroo/kanata) configuration files.

### Features

- Auto-detection of kanata files by .kbd file extension
- Syntax highlighting (colors keywords, action identifiers, etc.)
- Parse error checking. It will run after Ctrl+S is pressed and if there are any errors they will be underlined in editor and the error message will be shown.
- Support for [`includes`](https://github.com/jtroo/kanata/blob/main/docs/config.adoc#include). It's disabled by default but can be enabled in extension settings.

### Known issues and limitations

- Absolute paths in `include` blocks that point outside the opened workspace aren't supported.

### Bug reports
If you encounter a bug, please report it here: https://github.com/rszyma/vscode-kanata/issues

### Changelog

##### 0.1.0 (initial release) (2023-08-10)

- Added syntax highlighting.
- Added parse errors checking.
- Added support for includes.
- Used `kanata-parser` version: [3ccbcd1](https://github.com/jtroo/kanata/commits/3ccbcd1a2c8e482d4b2b1df1ce391934d43043d4)
