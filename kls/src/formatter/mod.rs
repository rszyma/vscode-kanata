pub mod ext_tree;
use ext_tree::*;

mod remove_excessive_newlines;
mod use_defsrc_layout_on_deflayers;

pub struct Formatter {
    // Additional options
    pub options: crate::ExtensionFormatterOptions,

    pub remove_extra_empty_lines: bool,
}

impl Formatter {
    /// Transforms [`ExtParseTree`] to another [`ExtParseTree`] in-place,
    /// applying selected formatting funtions depending on [`FormatterOptions`].
    pub fn format(&self, tree: &mut ExtParseTree) {
        if !self.options.enable {
            return;
        }
        if self.remove_extra_empty_lines {
            tree.0.remove_excessive_adjacent_newlines(2);
        }
        if self.options.use_defsrc_layout_on_deflayers {
            tree.use_defsrc_layout_on_deflayers()
        }
    }

    // VSCode normally sends formatting options along with
    // every textDocument/formatting request.
    pub fn format_with_options(
        &self,
        tree: &mut ExtParseTree,
        _options: &lsp_types::FormattingOptions, // todo: we should probably handle these options
    ) {
        self.format(tree)
    }
}
