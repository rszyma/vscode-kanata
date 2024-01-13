use std::ops::Deref;

use super::ext_tree::*;
use crate::log;
use unicode_segmentation::*;

impl ExtParseTree {
    // todo: maybe don't format if an atom in defsrc/deflayer is too large.
    pub fn use_defsrc_layout_on_deflayers<'a>(&'a mut self, tab_size: u32, insert_spaces: bool) {
        let mut defsrc: Option<&'a NodeList> = None;
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

            match first_atom.as_str() {
                "defsrc" => match defsrc {
                    Some(_) => {
                        log!(
                            "Formatting `deflayer`s failed: config file \
                                contains multiple `defsrc` definitions."
                        );
                        return;
                    }
                    None => {
                        defsrc = Some(top_level_list);
                    }
                },
                "deflayer" => {
                    deflayers.push(top_level_list);
                }
                "include" => {
                    // TODO: search defsrc in other files
                    // TODO: search defsrc in main file if the current one is included
                }
                _ => {}
            }
        }

        let defsrc = if let Some(x) = &mut defsrc {
            x
        } else {
            log!(
                "Formatting `deflayer`s failed: `defsrc` not found in this file. \
                NOTE: includes (or the main file, if this file is non-main) haven't \
                been checked, because it's not implemented yet."
            );
            return;
        };

        // Get number of atoms from `defsrc` now to prevent additional allocations
        // for `layout` later.
        // -1 because we don't count `defsrc` token.
        let defsrc_item_count: usize = defsrc.len() - 1;

        let mut layout: Vec<Vec<usize>> = vec![vec![0]; defsrc_item_count];

        // Read the layout from `defsrc`
        for (i, defsrc_item) in defsrc.iter().skip(1).enumerate() {
            if let Expr::List(_) = defsrc_item.expr {
                log!(
                    "Formatting `deflayer`s failed: there shouldn't \
                    be any lists in `defsrc`."
                );
                return;
            }

            let defsrc_item_as_str = defsrc_item.expr.to_string();

            let mut line_num: usize = 0;
            for ch in defsrc_item_as_str.chars() {
                match ch {
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
                    Metadata::Comment(_) => {
                        log!("formatting with comments in `defsrc` is unsupported for now");
                    }
                    Metadata::Whitespace(whitespace) => {
                        let mut line_num: usize = 0;
                        for ch in whitespace.chars() {
                            match ch {
                                '\n' => {
                                    layout[i].push(0);
                                    line_num += 1;
                                }
                                '\t' => layout[i][line_num] += tab_size as usize,
                                _ => layout[i][line_num] += 1_usize,
                            }
                        }
                    }
                }
            }
        }

        // Layout no longer needs to be mutable.
        let layout = layout;

        // Apply the `defsrc` layout to each `deflayer` block.
        for deflayer in &mut deflayers.iter_mut() {
            if deflayer.len() - 2 != defsrc_item_count {
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

            let last_expr_index = deflayer.len() - 3;
            for (i, deflayer_item) in deflayer.iter_mut().skip(2).enumerate() {
                let expr_graphemes_count = deflayer_item.expr.to_string().graphemes(true).count();

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
                    &layout[i],
                    &comments,
                    is_the_last_expr_in_deflayer,
                    // insert_spaces,
                );
                deflayer_item.post_metadata = new_post_metadata;
            }
        }
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
) -> Vec<Metadata> {
    if comments.is_empty() {
        formatted_deflayer_node_metadata_without_comments(
            expr_graphemes_count,
            formatting_to_apply,
            is_the_last_expr_in_deflayer,
        )
    } else {
        let indent = *formatting_to_apply.get(1).unwrap_or(&0);
        collect_comments_into_metadata_vec(comments, indent, is_the_last_expr_in_deflayer)
    }

    // return match comments[0] {
    //     Comment::LineComment(_) => collect_comments_into_metadata_vec(comments, indent),
    //     Comment::BlockComment(_) => {
    //         todo!()
    //     }
    // };
}

fn formatted_deflayer_node_metadata_without_comments(
    expr_graphemes_count: usize,
    formatting_to_apply: &[usize],
    is_the_last_expr_in_deflayer: bool,
) -> Vec<Metadata> {
    let mut result = if expr_graphemes_count < formatting_to_apply[0] {
        // Expr fits inside slot.
        vec![Metadata::Whitespace(
            " ".repeat(formatting_to_apply[0] - expr_graphemes_count),
        )]
    } else {
        // Expr doesn't fit inside slot, but it's not at the end of line, we just
        // add 1 space to separate from next expr.
        if !is_the_last_expr_in_deflayer {
            vec![Metadata::Whitespace(" ".to_string())]
        } else {
            vec![]
        }
    };

    for i in &formatting_to_apply[1..] {
        let mut s = "\n".to_string();
        for _ in 0..formatting_to_apply[1 + i] {
            s.push(' ');
        }
        result.push(Metadata::Whitespace(s));
    }

    result
}

fn collect_comments_into_metadata_vec(
    comments: &[&Comment],
    indent: usize,
    is_the_last_expr_in_deflayer: bool,
) -> Vec<Metadata> {
    let mut result: Vec<Metadata> = vec![Metadata::Whitespace(" ".to_string())];

    for (i, comment) in comments.iter().enumerate() {
        let is_last_comment: bool = i + 1 == comments.len();
        result.push(Metadata::Comment(comment.deref().clone()));
        match comment {
            Comment::LineComment(_) => {
                if is_last_comment {
                    if !is_the_last_expr_in_deflayer {
                        result.push(Metadata::Whitespace(" ".to_string()));
                    }
                } else {
                    result.push(Metadata::Whitespace(" ".repeat(indent)));
                }
            }
            Comment::BlockComment(_) => {
                // FIXME: a space shoudln't be added if it's the last item in `deflayer`.
                result.push(Metadata::Whitespace(" ".to_string()));
            }
        };
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_defsrc_layout_on_deflayers() {
        let cases = [
            (
                // empty file - no changes
                "", "",
            ),
            (
                // no defsrc - no changes
                "( defcfg )",
                "( defcfg )",
            ),
            (
                // 1 deflayer, no defsrc - no changes
                "(deflayer base  1 2 )",
                "(deflayer base  1 2 )",
            ),
            (
                // 1 defsrc, 1 deflayer (very simple) - formatting applied
                "(defsrc 1 2) (deflayer base 3  4)",
                "(defsrc 1 2) (deflayer base 3 4)",
            ),
            (
                // 1 defsrc, 1 deflayer - formatting applied
                "(defsrc \n 1  2\n) (deflayer base 3 4 )",
                "(defsrc \n 1  2\n) (deflayer base 3  4\n)",
            ),
            (
                // format applies to all deflayers
                "(defsrc \n 1  2\n) (deflayer base 1 2 ) ( deflayer\n\t layer2 \n\n3  \t  \n  \t4\n )",
                "(defsrc \n 1  2\n) (deflayer base 1  2\n) ( deflayer\n\t layer2 \n\n3  4\n)",
            ),
            (
                // format doesn't apply to blocks other than `deflayer`
                "(defsrc \n 1  2\n)  (\ndefalias\n\ta b\n)  (deflayer base 1 2 )  ( deflayer \n\t layer2 \n\n3   4 )",
                "(defsrc \n 1  2\n)  (\ndefalias\n\ta b\n)  (deflayer base 1  2\n)  ( deflayer \n\t layer2 \n\n3  4\n)",
            ),
            (
                /*
                1 defsrc, 1 correct deflayer + 1 deflayer with wrong number of
                items - formatting applied only to the correct deflayer.
                */
                "(defsrc \n 1  2\n)  (deflayer wrong 1 2  3)  ( deflayer\n\t right \n\n3   4 )",
                "(defsrc \n 1  2\n)  (deflayer wrong 1 2  3)  ( deflayer\n\t right \n\n3  4\n)",
            ),
            (
                // format works when config has multi-byte unicode character
                "(defsrc \n 0 1  2\n)  (deflayer base 🌍   1  \n 2 \t)",
                "(defsrc \n 0 1  2\n)  (deflayer base 🌍 1  2\n)",
            ),
            (
                // format works when config has multi-cluster unicode characters
                "(defsrc \n 0 1  2\n)  (deflayer base 👨‍👩‍👧‍👦 \t 1     2 \n\n)",
                "(defsrc \n 0 1  2\n)  (deflayer base 👨‍👩‍👧‍👦 1  2\n)",
            ),
            (
                // no format when defsrc uses an (invalid) list item
                "(defsrc () 1  2)  (deflayer base 0 1 2)",
                "(defsrc () 1  2)  (deflayer base 0 1 2)",
            ),
            (
                // fix: newline in deflayer not removed
                "(defsrc 1  2) (deflayer base 3  4\n)",
                "(defsrc 1  2) (deflayer base 3  4)",
            ),
        ];

        let _ignored_cases = [
            (
                /*
                Currently formatter does this:
                   "(defsrc caps w a s d) (deflayer mouse  1    2 3 4 5)",

                But it seems better if formatting was forced to 1 space if each item
                is only a space apart from each other in defsrc:
                   "(defsrc caps w a s d) (deflayer mouse   1     2   3 4   5)",
                   "(defsrc caps w a s d) (deflayer mouse 1 2 3 4 5)",

                */
                "(defsrc caps w a s d) (deflayer mouse 1    2 3 4 5)",
                "(defsrc caps w a s d) (deflayer mouse 1 2 3 4 5)",
            ),
            (
                /*
                1 defsrc, 1 deflayer, but a line comment inside defsrc - formatting applied
                FIXME: should the space before closing paren stay?
                */
                "(defsrc 1  2 # Mary had a little lamb\n)  (deflayer base 1 2)",
                "(defsrc 1  2 # Mary had a little lamb\n)  (deflayer base 1  2 )",
            ),
            (
                /*
                1 defsrc, 1 deflayer, but a line comment inside a deflayer - no
                formatting applied.
                FIXME: investigate how this feels, and possibly change.
                */
                "(defsrc 1  2)  (deflayer base\n  1\n  # Mary had a little lamb\n  2)",
                "(defsrc 1  2)  (deflayer base 1 # Mary had a little lamb\n  2)",
            ),
            (
                // TODO: same as the two above, but for block comments.
                "", "",
            ),
            (
                // Convert line comment to block comment if formatting
                "", "",
            ),
        ];

        for (i, (case, expected_result)) in cases.iter().enumerate() {
            let mut tree = parse_into_ext_tree(case).expect("parses");
            tree.use_defsrc_layout_on_deflayers(4, true);
            assert_eq!(
                tree.to_string(),
                *expected_result,
                "parsed tree did not equal to expected_result"
            );
        }
    }
}
