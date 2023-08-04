# Kanata configuration language support VS Code

Language support for .kbd kanata (https://github.com/jtroo/kanata) configuration files.

### Features

- syntax highlighting
- syntax validation (shows errors and their position)

### Settings

- If you want use `include` blocks in your kanata config files you need to enable the support for them in VS Code settings under "Kanata" category.

### Known issues and limitations

- Absolute paths in `include` blocks that point outside the opened workspace aren't supported.

### Changelog

##### 0.1.0 (initial release) (2023-08-04)

- Add syntax highlighting.
- Add syntax checking in workspace mode.