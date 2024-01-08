use crate::log;
use kanata_parser::cfg::{
    sexpr::{self, Position, SExpr, SExprMetaData, Span, Spanned},
    ParseError,
};
use std::{borrow::BorrowMut, collections::HashMap};
use wasm_bindgen::prelude::*;

pub mod ext_tree;
use ext_tree::*;

mod remove_excessive_newlines;

pub struct Formatter {
    // Additional options
    pub extension_options: crate::ExtensionFormatterOptions,

    pub remove_extra_empty_lines: bool,
}

impl Formatter {
    /// Transforms [`ExtParseTree`] to another [`ExtParseTree`] in-place,
    /// applying selected formatting funtions depending on [`FormatterOptions`].
    pub fn format(&self, tree: &mut ExtParseTree) {
        if !self.extension_options.enable {
            return;
        }
        if self.remove_extra_empty_lines {
            tree.0.remove_excessive_adjacent_newlines(2);
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
