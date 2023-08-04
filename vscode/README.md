# Kanata configuration language support VS Code

Language support for .kbd kanata (https://github.com/jtroo/kanata) configuration files.

### Features

- syntax highlighting
- syntax checking (shows errors and their position)

### Known issues / current limitations

- Syntax checking currently only works when.  `kanata.kbd`.
- Absolute paths in `include` action that point outside current workspace are supported.

### Changelog

##### 0.1.0 (initial release) (2023-08-04)

- Add syntax highlighting.
- Add syntax checking in workspace mode.