use std::fmt::Display;

use super::ext_tree::*;
use crate::log;
use anyhow::anyhow;
use unicode_segmentation::*;

pub mod get_keys;
pub use get_keys::*;
pub mod get_layout;
pub use get_layout::*;

impl ExtParseTree {
    // TODO: maybe don't format if an atom in defsrc/deflayer is too large.
    // TODO: respect `tab_size`.
    // TODO: respect `insert_spaces` formatter setting.
    pub fn use_defsrc_layout_on_deflayers<'a>(
        &'a mut self,
        defsrc_layout: &[Vec<usize>],
        _tab_size: u32,
        _insert_spaces: bool,
        line_ending: LineEndingSequence,
    ) {
        let mut deflayers: Vec<&'a mut NodeList> = vec![];

        for top_level_item in self.0.iter_mut() {
            let top_level_list = match &mut top_level_item.expr {
                Expr::Atom(_) => continue,
                Expr::List(list) => list,
            };

            let first_item = match top_level_list.get(0) {
                Some(x) => x,
                None => continue,
            };

            let first_atom = match &first_item.expr {
                Expr::Atom(x) => x,
                Expr::List(_) => continue,
            };

            if let "deflayer" = first_atom.as_str() {
                deflayers.push(top_level_list);
            }
        }

        // Apply the `defsrc` layout to each valid `deflayer` block.
        for deflayer in &mut deflayers.iter_mut() {
            if deflayer.len() <= 2 {
                // At least a "deflayer" token and layer name is needed for valid deflayer block.
                continue;
            }

            if deflayer.len() - 2 != defsrc_layout.len() {
                let layer_name = deflayer
                    .get(1)
                    .map(|f| if let Expr::Atom(x) = &f.expr { x } else { "?" })
                    .unwrap_or("?");
                log!(
                    "Formatting of '{}' deflayer skipped: item count doesn't match defsrc",
                    layer_name
                );
                continue;
            }

            // trim spaces at the end of each line in deflayer
            let defsrc_layout: Vec<Vec<usize>> = {
                let mut layout = defsrc_layout.to_vec();
                for x in layout.iter_mut() {
                    // at least 2 items means that it's newline
                    if x.len() >= 2 {
                        if let Some(first) = x.get_mut(0) {
                            *first = 1;
                        }
                    }
                }
                layout
            };

            let last_expr_index = deflayer.len() - 3;
            for (i, deflayer_item) in deflayer.iter_mut().skip(2).enumerate() {
                let expr_graphemes_count = deflayer_item.expr.to_string().graphemes(true).count();

                // NOTE: we're ignoring `pre_metadata` here on purpose.
                // We're assuming that the passed ExtTree has not been modified after parsing,
                // and when first parsing we only save comments in `post_medatata`.
                let post_metadata: Vec<_> = deflayer_item.post_metadata.drain(..).collect();

                let comments: Vec<_> = post_metadata
                    .iter()
                    .filter_map(|md| match md {
                        Metadata::Comment(x) => Some(x),
                        Metadata::Whitespace(_) => None,
                    })
                    .collect();

                let is_the_last_expr_in_deflayer = i == last_expr_index;

                let new_post_metadata = formatted_deflayer_node_metadata(
                    expr_graphemes_count,
                    &defsrc_layout[i],
                    &comments,
                    is_the_last_expr_in_deflayer,
                    line_ending,
                );
                deflayer_item.post_metadata = new_post_metadata;
            }
        }
    }

    /// Obtains defsrc layout from a given [`ExtParseTree`].
    /// * It doesn't search includes.
    /// * Returns `Err` if found more than 1 defsrc, or `defsrc` contains a list.
    /// * Returns `Ok(None)` if found 0 defsrc blocks.
    /// * Returns `Ok(Some)` otherwise.
    pub fn defsrc_layout<'a>(&'a self, tab_size: u32) -> anyhow::Result<Option<Vec<Vec<usize>>>> {
        let mut defsrc: Option<&'a NodeList> = None;

        for top_level_item in self.0.iter() {
            let top_level_list = match &top_level_item.expr {
                Expr::Atom(_) => continue,
                Expr::List(list) => list,
            };

            let first_item = match top_level_list.get(0) {
                Some(x) => x,
                None => continue,
            };

            let first_atom = match &first_item.expr {
                Expr::Atom(x) => x,
                Expr::List(_) => continue,
            };

            if let "defsrc" = first_atom.as_str() {
                match defsrc {
                    Some(_) => {
                        return Err(anyhow!("multiple `defsrc` definitions in a single file"));
                    }
                    None => {
                        defsrc = Some(top_level_list);
                    }
                }
            }
        }

        let defsrc = match defsrc {
            Some(x) => x,
            None => {
                // defsrc not found in this file, but it may be in another.
                return Ok(None);
            }
        };

        // Get number of atoms from `defsrc` now to prevent additional allocations
        // for `layout` later.
        // -1 because we don't count `defsrc` token.
        let defsrc_item_count: usize = defsrc.len() - 1;

        let mut layout: Vec<Vec<usize>> = vec![vec![0]; defsrc_item_count];

        // Read the layout from `defsrc`
        for (i, defsrc_item) in defsrc.iter().skip(1).enumerate() {
            if let Expr::List(_) = defsrc_item.expr {
                return Err(anyhow!("found a list in `defsrc`"));
            }

            let defsrc_item_as_str = defsrc_item.expr.to_string();

            let mut line_num: usize = 0;
            for ch in defsrc_item_as_str.chars() {
                match ch {
                    '\r' => {}
                    '\n' => {
                        layout[i].push(0);
                        line_num += 1;
                    }
                    '\t' => layout[i][line_num] += tab_size as usize,
                    _ => layout[i][line_num] += 1_usize,
                }
            }

            // NOTE: We intentionally process only `post_metadata` and ignore `pre_metadata`.
            // This should be either fixed later, or we just shouldn't modify `pre_metadata`
            // in previous operations on tree.
            for metadata in &defsrc_item.post_metadata {
                match metadata {
                    Metadata::Comment(comment) => {
                        if let Comment::LineComment(_) = comment {
                            layout[i].push(0);
                            line_num += 1;
                        }
                    }
                    Metadata::Whitespace(whitespace) => {
                        for ch in whitespace.chars() {
                            match ch {
                                '\r' => {}
                                '\n' => {
                                    layout[i].push(0);
                                    line_num += 1;
                                }
                                '\t' => layout[i][line_num] += tab_size as usize,
                                ' ' => layout[i][line_num] += 1_usize,
                                _ => unreachable!(),
                            }
                        }
                    }
                }
            }
        }

        // Layout no longer needs to be mutable.
        Ok(Some(layout))
    }

    /// Obtains list of keys in defsrc block in given [`ExtParseTree`].
    /// * It doesn't search includes.
    /// * Returns `Err` if found more than 1 defsrc, or `defsrc` contains a list.
    /// * Returns `Ok(None)` if found 0 defsrc blocks.
    /// * Returns `Ok(Some)` otherwise.
    pub fn defsrc_keys<'a>(&'a self) -> anyhow::Result<Option<Vec<String>>> {
        let mut defsrc: Option<&'a NodeList> = None;

        for top_level_item in self.0.iter() {
            let top_level_list = match &top_level_item.expr {
                Expr::Atom(_) => continue,
                Expr::List(list) => list,
            };

            let first_item = match top_level_list.get(0) {
                Some(x) => x,
                None => continue,
            };

            let first_atom = match &first_item.expr {
                Expr::Atom(x) => x,
                Expr::List(_) => continue,
            };

            if let "defsrc" = first_atom.as_str() {
                match defsrc {
                    Some(_) => {
                        return Err(anyhow!("multiple `defsrc` definitions in a single file"));
                    }
                    None => {
                        defsrc = Some(top_level_list);
                    }
                }
            }
        }

        let defsrc = match defsrc {
            Some(x) => x,
            None => {
                // defsrc not found in this file, but it may be in another.
                return Ok(None);
            }
        };

        let result: Vec<String> = defsrc
            .iter()
            .skip(1)
            .map(|x| {
                Ok(match &x.expr {
                    Expr::List(_) => return Err(anyhow!("found a list in `defsrc`")),
                    Expr::Atom(x) => x.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(result))
    }
}

/// Format metadata for a definition layer node based on specified constraints.
///
/// # Arguments
///
/// * `expr_graphemes_count` - Represents the minimum amount of space that metadata needs to occupy.
/// * `formatting_to_apply` - Each item represents the number of spaces between '\n' characters.
/// * `comments` - Comment metadata attached after the `Expr`.
///
/// # Returns
///
/// A vector containing formatted metadata.
fn formatted_deflayer_node_metadata(
    expr_graphemes_count: usize,
    formatting_to_apply: &[usize],
    comments: &[&Comment],
    is_the_last_expr_in_deflayer: bool,
    line_ending: LineEndingSequence,
) -> Vec<Metadata> {
    if comments.is_empty() {
        formatted_deflayer_node_metadata_without_comments(
            expr_graphemes_count,
            formatting_to_apply,
            is_the_last_expr_in_deflayer,
            line_ending,
        )
    } else {
        let indent = formatting_to_apply.get(1).copied();
        collect_comments_into_metadata_vec(
            comments,
            indent,
            is_the_last_expr_in_deflayer,
            line_ending,
        )
    }
}

fn formatted_deflayer_node_metadata_without_comments(
    expr_graphemes_count: usize,
    formatting_to_apply: &[usize],
    is_the_last_expr_in_deflayer: bool,
    line_ending: LineEndingSequence,
) -> Vec<Metadata> {
    let is_at_the_end_of_line = formatting_to_apply.len() >= 2;
    #[allow(clippy::nonminimal_bool)]
    let mut result =
        if is_at_the_end_of_line || (!is_at_the_end_of_line && is_the_last_expr_in_deflayer) {
            // space-before-newline / space-before-end-paren trimming
            vec![]
        } else if expr_graphemes_count < formatting_to_apply[0] {
            // Expr fits inside slot.
            let n = formatting_to_apply[0] - expr_graphemes_count;
            if n > 0 {
                vec![Metadata::Whitespace(" ".repeat(n))]
            } else {
                vec![]
            }
        } else {
            // Expr doesn't fit inside slot, but it's not at the end of line, we just
            // add 1 space to separate from next expr.
            if !is_the_last_expr_in_deflayer {
                vec![Metadata::Whitespace(" ".to_string())]
            } else {
                vec![]
            }
        };

    for n in &formatting_to_apply[1..] {
        let mut s = line_ending.to_string();
        for _ in 0..*n {
            s.push(' ');
        }
        result.push(Metadata::Whitespace(s));
    }

    result
}

fn collect_comments_into_metadata_vec(
    comments: &[&Comment],
    next_line_indent: Option<usize>,
    is_the_last_expr_in_deflayer: bool,
    line_ending: LineEndingSequence,
) -> Vec<Metadata> {
    // non-empty comments vec should be passed, but we're handling it anyways
    if comments.is_empty() {
        if next_line_indent.is_some() {
            return vec![Metadata::Whitespace(line_ending.to_string())];
        } else {
            return vec![];
        }
    }
    let mut result: Vec<Metadata> = vec![Metadata::Whitespace(" ".to_string())];

    for (i, comment) in comments.iter().enumerate() {
        let is_the_last_comment: bool = i + 1 == comments.len();
        result.push(Metadata::Comment((*comment).clone()));
        match comment {
            Comment::LineComment(_) => {
                if !is_the_last_expr_in_deflayer {
                    result.push(Metadata::Whitespace(
                        " ".repeat(next_line_indent.unwrap_or(0)),
                    ));
                }
            }
            Comment::BlockComment(_) => match next_line_indent {
                Some(indent) => {
                    if is_the_last_comment {
                        result.push(Metadata::Whitespace(line_ending.to_string()));
                        if !is_the_last_expr_in_deflayer {
                            result.push(Metadata::Whitespace(" ".repeat(indent)));
                        }
                    } else if !is_the_last_expr_in_deflayer {
                        result.push(Metadata::Whitespace(" ".to_string()));
                    }
                }
                None => {
                    if !is_the_last_expr_in_deflayer {
                        result.push(Metadata::Whitespace(" ".to_string()));
                    }
                }
            },
        };
    }

    result
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
pub enum LineEndingSequence {
    LF,
    #[allow(dead_code)]
    CRLF,
}

impl Display for LineEndingSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineEndingSequence::LF => f.write_str("\n"),
            LineEndingSequence::CRLF => f.write_str("\r\n"),
        }?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formats_correctly(input: &str, expected_output: &str) {
        _formats_correctly(input, expected_output)
            .expect("should not error in order to format correctly")
    }

    fn _formats_correctly(input: &str, expected_output: &str) -> anyhow::Result<()> {
        let mut tree = parse_into_ext_tree(input).expect("parses");
        let tab_size = 4;

        let defsrc_layout = tree
            .defsrc_layout(tab_size)
            .expect("no err")
            .ok_or(anyhow!("expected Some"))?;
        dbg!(&defsrc_layout);

        tree.use_defsrc_layout_on_deflayers(&defsrc_layout, tab_size, true, LineEndingSequence::LF);

        assert_eq!(
            tree.to_string(),
            expected_output,
            "parsed tree did not equal to expected_result"
        );

        Ok(())
    }

    fn should_not_format(input: &str) {
        match _formats_correctly(input, input) {
            Ok(_) => {}
            Err(_) => {
                // if it fails, it won't be formatted (I hope?)
            }
        }
    }

    #[test]
    fn empty_file_no_changes() {
        should_not_format("");
    }

    #[test]
    fn just_defsrc_no_other_blocks() {
        should_not_format("( defcfg )");
    }

    #[test]
    fn deflayer_defined_but_no_defsrc() {
        should_not_format("(deflayer base  1 2 )");
    }

    #[test]
    fn some_simple_cases() {
        formats_correctly(
            "(defsrc \n 1  2\n) (deflayer base 3 4 )",
            "(defsrc \n 1  2\n) (deflayer base 3  4\n)",
        );
        // TODO: how should we format in such a simple case?
        // While this seems the right this to do:
        formats_correctly(
            "(defsrc 1 2) (deflayer base 3  4)",
            "(defsrc 1 2) (deflayer base 3 4)",
        );
        // the following looks a bit weird, but it's the current
        // formatter behavior:
        formats_correctly(
            "(defsrc caps a) (deflayer base 1 2)",
            "(defsrc caps a) (deflayer base 1    2)",
        );
    }

    #[test]
    fn multiple_deflayers() {
        formats_correctly(
            "(defsrc \n 1  2\n) (deflayer base 1 2 ) ( deflayer\n\t layer2 \n\n3  \t  \n  \t4\n )",
            "(defsrc \n 1  2\n) (deflayer base 1  2\n) ( deflayer\n\t layer2 \n\n3  4\n)",
        );
    }

    #[test]
    fn only_deflayer_blocks_get_formatted() {
        formats_correctly(
            "(defsrc \n 1  2\n)  (\ndefalias\n\ta b\n)  (deflayer base 1 2 )  ( deflayer \n\t layer2 \n\n3   4 )",
            "(defsrc \n 1  2\n)  (\ndefalias\n\ta b\n)  (deflayer base 1  2\n)  ( deflayer \n\t layer2 \n\n3  4\n)",
        );
    }

    #[test]
    fn wrong_number_of_items_in_one_of_deflayers() {
        // Formatting should apply only to the correct deflayers, while skipping the incorrect ones.
        formats_correctly(
            "(defsrc \n 1  2\n)  (deflayer wrong 1 2  3)  ( deflayer\n\t right \n\n3   4 )",
            "(defsrc \n 1  2\n)  (deflayer wrong 1 2  3)  ( deflayer\n\t right \n\n3  4\n)",
        );
    }

    #[test]
    fn multi_byte_unicode_chars() {
        formats_correctly(
            "(defsrc \n 0 1  2\n)  (deflayer base 🌍   1  \n 2 \t)",
            "(defsrc \n 0 1  2\n)  (deflayer base 🌍 1  2\n)",
        );
    }

    #[test]
    fn multi_cluster_unicode_chars() {
        formats_correctly(
            "(defsrc \n 0 1  2\n)  (deflayer base 👨‍👩‍👧‍👦 \t 1     2 \n\n)",
            "(defsrc \n 0 1  2\n)  (deflayer base 👨‍👩‍👧‍👦 1  2\n)",
        );
    }

    #[test]
    fn defsrc_layout_when_invalid_list_item_in_defsrc() {
        let input = "(defsrc () 1  2)  (deflayer base 0 1 2)";
        let tree = parse_into_ext_tree(input).expect("parses");
        tree.defsrc_layout(4).expect_err(
            "should error, because there's a list item in defsrc, \
            which is an error in kanata config",
        );
    }

    #[test]
    fn extra_newlines_at_the_end_of_deflayer_get_removed() {
        formats_correctly(
            "(defsrc 1  2) (deflayer base 3  4\n)",
            "(defsrc 1  2) (deflayer base 3  4)",
        );
    }

    #[test]
    fn spaces_at_the_end_of_each_line_in_deflayer_get_removed() {
        formats_correctly(
            "(defsrc caps\nbspc\npgup\npgdn\n) (deflayer base 3  \n4 \ngrv\n6\n)",
            "(defsrc caps\nbspc\npgup\npgdn\n) (deflayer base 3\n4\ngrv\n6\n)",
        );
    }

    #[test]
    fn line_comment_in_deflayer() {
        // regression test for the bug: wrong spacing in the newline after line
        // comment on last line
        should_not_format("(defsrc 1  2 \n  3) (deflayer base 4  5 ;;\n  6)");

        // Both cases seem correct, but only the first one passes as of now.
        // idk how to fix this. Probably another arg would need to be
        // added to `formatted_deflayer_node_metadata` or something.
        should_not_format("(defsrc 1  2\n) (deflayer base 4  5 ;;\n)");
        // should_not_format("(defsrc 1  2\n) (deflayer base 4  5\n;;\n)");
    }

    #[test]
    fn indent_of_a_line_after_a_line_comment_is_correct() {
        // should pass with just newline
        should_not_format("(defsrc 1  2\n  3) (deflayer base 4  5\n  6)");
        // and also with line comment
        should_not_format("(defsrc 1  2 \n  3) (deflayer base 4  5 ;;\n  6)");
        // https://github.com/rszyma/vscode-kanata/issues/15
        formats_correctly(
            "(defsrc\n  a b c\n)\n(deflayer base\n  a b ;;\n  c\n)",
            "(defsrc\n  a b c\n)\n(deflayer base\n  a b ;;\nc\n)",
        );
    }

    #[test]
    fn block_comment_in_deflayer() {
        // at the end of `deflayer`
        should_not_format("(defsrc 1  2) (deflayer base 4  5 #||#)");
        // between items
        should_not_format("(defsrc 1  2) (deflayer base 4 #||# 5)");
        // between items, before newline
        should_not_format("(defsrc 1\n2) (deflayer base 4 #||#\n5)");
        // between items, before newline, respecting indent after newline
        should_not_format("(defsrc 1\n  2) (deflayer base 4 #||#\n  5)");
    }

    #[test]
    #[ignore = "not implemented"]
    fn no_format_when_defsrc_has_no_extra_spacing() {
        /*
        Currently formatter does this:
           "(defsrc caps w a s d) (deflayer base 1    2 3 4 5)",
        but the idea is to not add additional padding after "1" if defsrc
        has no spaces (or newlines) itself
        */
        should_not_format("(defsrc caps w a s d) (deflayer base 1 2 3 4 5)");
        // but extra spacing between items in deflayer should still apply:
        formats_correctly(
            "(defsrc caps w a s d) (deflayer base 1 2   3 4   5)",
            "(defsrc caps w a s d) (deflayer base 1 2 3 4 5)",
        );
    }

    #[test]
    fn cr_lf_line_endings() {
        // Tests 2 things:
        // 1. No panic (see https://github.com/rszyma/vscode-kanata/issues/20)
        // 2. CRLF line endings applying correctly.

        let [input, expected_output] = [
            "(defsrc 1 \r\n 2)  (deflayer base 3 \r\n 4)",
            "(defsrc 1 \r\n 2)  (deflayer base 3\r\n 4)", // one whitespace less because of eol space trimming
        ];

        let mut tree = parse_into_ext_tree(input).expect("parses");
        let tab_size = 4;
        if let Some(layout) = tree.defsrc_layout(tab_size).expect("no err") {
            tree.use_defsrc_layout_on_deflayers(&layout, tab_size, true, LineEndingSequence::CRLF);
        };
        assert_eq!(
            tree.to_string(),
            expected_output,
            "parsed tree did not equal to expected_result"
        );
    }

    #[test]
    fn test_defsrc_layout() {
        let input = "(defsrc 1 \r\n 2\t 3)";
        let tree = parse_into_ext_tree(input).expect("parses");

        let tab_size = 4;
        let output = tree
            .defsrc_layout(tab_size)
            .expect("no err")
            .expect("is Some");

        let expected_output = vec![
            vec![2, 1], // "2" because "1 " is 2 chars, and 1 is a single space after crlf
            vec![6],    // "2\t " -- 1 from char + 4 from tab + 1 space -> 6
            vec![1],    // "3" has len 1
        ];

        assert_eq!(
            output, expected_output,
            "parsed tree did not equal to expected_result"
        );
    }

    /// Regression test for https://github.com/rszyma/vscode-kanata/issues/51
    #[test]
    fn test_deflayer_slot_expand() {
        should_not_format("(defsrc 1 2 3 4) (deflayer base 1 @a grv 4)");
    }
}
