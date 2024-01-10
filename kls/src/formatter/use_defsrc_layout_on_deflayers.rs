use crate::log;

use super::ext_tree::*;

impl ExtParseTree {
    // todo: maybe don't format if an atom in defsrc/deflayer is too large.
    pub fn use_defsrc_layout_on_deflayers(&mut self) {
        let mut defsrc_node_path: Option<Vec<usize>> = None;
        let mut deflayer_node_paths: Vec<Vec<usize>> = vec![];
        for (i, top_level_item) in self.unwrap_list().iter().enumerate() {
            // Find first list or atom.
            for node in top_level_item.list_or_default() {
                match node {
                    ParseTreeNode::Atom(x) => match x.as_str() {
                        "defsrc" => match defsrc_node_path {
                            Some(_) => {
                                log!(
                                    "Formatting `deflayer`s failed: config file \
                                    contains multiple `defsrc` blocks."
                                );
                                return;
                            }
                            None => {
                                defsrc_node_path = Some(vec![i]);
                                break;
                            }
                        },
                        "deflayer" => {
                            deflayer_node_paths.push(vec![i]);
                            break;
                        }
                        _ => break,
                    },
                    ParseTreeNode::BlockComment(_)
                    | ParseTreeNode::LineComment(_)
                    | ParseTreeNode::Whitespace(_) => continue,
                    ParseTreeNode::List(_) => {
                        break;
                    }
                }
            }
        }

        let defsrc_node_path = if let Some(x) = defsrc_node_path {
            x
        } else {
            log!(
                "Formatting `deflayer`s failed: `defsrc` not found in this file. \
                NOTE: includes (or the main file, if this file is non-main) haven't \
                been checked, because it's not implemented yet."
            );
            return;
        };

        // This no longer needs to be mutable.
        let deflayer_node_paths = deflayer_node_paths;

        let defsrc_node = self.get_node(&defsrc_node_path).unwrap();

        // Get number of atoms from `defsrc` now to prevent additional allocations
        // for `layout` later.

        let defsrc_atom_count: usize = defsrc_node.unwrap_list().iter().fold(0, |acc, node| {
            if let ParseTreeNode::Atom(_) = node {
                acc + 1
            } else {
                acc
            }
        });

        // Each item in layout is a string that consists only of whitespace characters.
        let mut layout: Vec<String> = vec![String::with_capacity(10); defsrc_atom_count];

        let mut current_layout_item_index: usize = 0;

        // Read the layout from `defsrc`

        for node in defsrc_node.unwrap_list() {
            match node {
                ParseTreeNode::Atom(x) => {
                    current_layout_item_index += 1;
                    // If we didn't drop Spanned while merging `SExpr` with `SExprMetaData`
                    // we could use it to get size, but oh well.
                    let size = x.chars().count();
                    // Subtract 1, because atom/list in `deflayer` will always have at
                    // least 1 width.
                    for _ in 0..(size - 1) {
                        layout[current_layout_item_index].push(' ');
                    }
                }
                ParseTreeNode::Whitespace(x) => {
                    layout[current_layout_item_index].push_str(x);
                }
                ParseTreeNode::List(_) => {
                    log!(
                        "Formatting `deflayer`s failed: there shouldn't \
                        be any lists in `defsrc`."
                    );
                    return;
                }
                ParseTreeNode::LineComment(_) => {
                    // Treat line comment as newline. Since LineComment should
                    // contain a newline. (or not if at the end of file?).
                    // But this is inside a top-level block, so it's guaranteed
                    // that the comment contains a newline.
                    layout[current_layout_item_index].push('\n');
                }
                ParseTreeNode::BlockComment(_) => {
                    // Nothing special to do here.
                }
            }
        }

        // Layout no longer needs to be mutable.
        let layout = layout;

        // Modify deflayers according to layout.

        'outer: for deflayer_path in deflayer_node_paths {
            let deflayer_mut = self.get_node_mut(&deflayer_path).unwrap().unwrap_list_mut();

            let deflayer_atom_count: usize = deflayer_mut.iter().fold(0, |acc, node| {
                if let ParseTreeNode::Atom(_) = node {
                    acc + 1
                } else {
                    acc
                }
            });

            if deflayer_atom_count != defsrc_atom_count {
                continue;
            }

            let mut deflayer = deflayer_mut.iter_mut().peekable();
            let mut deflayer_index = 0;

            let mut layout = layout.iter();
            let mut atoms_to_ignore = 2;

            for layout_item in layout.by_ref() {
                // slot = atom/list + opt<space> + fill_remaining_with_space
                let minimum_slot_width: usize = layout_item.chars().count();
                let mut slot_width_occupied: usize = 0;

                let mut layout_item = layout_item.chars();

                'mid: loop {
                    let node = match deflayer.next_if(|next| {
                        return if slot_width_occupied >= minimum_slot_width {
                            match next {
                                ParseTreeNode::List(_) | ParseTreeNode::Atom(_) => false,
                                _ => true,
                            }
                        } else {
                            true
                        };
                    }) {
                        Some(x) => x,
                        None => {
                            break;
                        }
                    };

                    let node_width = node.width();
                    match node {
                        ParseTreeNode::List(_) | ParseTreeNode::Atom(_) => {
                            if atoms_to_ignore > 0 {
                                atoms_to_ignore -= 1;
                                continue;
                            }
                            slot_width_occupied += node_width;
                        }
                        ParseTreeNode::Whitespace(x) => {
                            while let Some(ch) = layout_item.next() {
                                if slot_width_occupied >= minimum_slot_width {
                                    if ch == '\n' {
                                        x.push('\n');
                                    }
                                    continue 'mid;
                                }
                                x.push(ch);
                                slot_width_occupied += 1;
                            }
                            // let remaining_whitespace_chars
                            // x.chars().
                        }
                        ParseTreeNode::LineComment(_) => {
                            // don't care about deflayers with line comment for now
                            // these are really problematic tbh, because we might want to
                            // insest a neww Whitespace item after it.
                            return;
                        }
                        ParseTreeNode::BlockComment(_) => {
                            slot_width_occupied += node_width;
                        }
                    }
                }

                if slot_width_occupied < minimum_slot_width {
                    let remaining = minimum_slot_width - slot_width_occupied;
                    let mut s = String::with_capacity(remaining);
                    for _ in 0..remaining {
                        s.push(' ')
                    }
                    deflayer_mut.push(ParseTreeNode::Whitespace(s));
                }
            }

            if deflayer.next().is_some() {
                panic!("items still left in deflayer")
            }
            if layout.next().is_some() {
                panic!("items still left in layout")
            }
        }

        // Apply the `defsrc` layout to each `deflayer` block.
    }
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
                // 1 defsrc, 1 deflayer - formatting applied
                "(defsrc \n 1  2\n)\
                 (deflayer base 1 2 )",
                "(defsrc \n 1  2\n)\
                 (deflayer base \n 1  2\n)",
            ),
            (
                // 1 defsrc, 2 deflayers - formatting applied for both deflayers
                "(defsrc \n 1  2\n)\
                 (deflayer base 1 2 )\
                 ( deflayer\n\t layer2 \n\n3   4 )",
                "(defsrc \n 1  2\n)\
                 (deflayer base \n 1  2\n)\
                 (deflayer layer2 \n 3  4\n)",
            ),
            (
                /*
                1 defsrc, 1 defalias, 2 deflayers - formatting applied for both deflayers,
                while defalias should be untouched.
                */
                "(defsrc \n 1  2\n)\
                 (\ndefalias\n\ta b\n)\
                 (deflayer base 1 2 )\
                 ( deflayer\n\t layer2 \n\n3   4 )",
                "(defsrc \n 1  2\n)\
                 (\ndefalias\n\ta b\n)\
                 (deflayer base \n 1  2\n)\
                 (deflayer layer2 \n 3  4\n)",
            ),
            (
                /*
                1 defsrc, 1 correct deflayer + 1 deflayer with wrong number of
                items - formatting applied only to the correct deflayer.
                */
                "(defsrc \n 1  2\n)\
                 (deflayer base 1 2 )\
                 ( deflayer\n\t layer2 \n\n3   4 )",
                "(defsrc \n 1  2\n)\
                 (deflayer base \n 1  2\n)\
                 (deflayer base \n 3  4\n)",
            ),
            (
                /*
                1 defsrc, 1 deflayer, but the deflayer contains a multi-byte unicode
                character (but no multi-cluster) - formatting should be applied
                correctly regardless of used characters.
                */
                "(defsrc \n 🌍 1  2\n)\
                 (deflayer base 🌍 1  2 )",
                "(defsrc \n 🌍 1  2\n)\
                 (deflayer base \n 🌍 1  2\n)",
            ),
            (
                /*
                1 defsrc, 1 deflayer, but the deflayer contains a multi-cluster
                unicode character - formatting should be applied correctly
                regardless of used characters.
                */
                "(defsrc \n 👨‍👩‍👧‍👦 1  2\n)\
                 (deflayer base 👨‍👩‍👧‍👦 1  2 )",
                "(defsrc \n 👨‍👩‍👧‍👦 1  2\n)\
                 (deflayer base \n 👨‍👩‍👧‍👦 1  2\n)",
            ),
            (
                // 1 invalid defsrc, 1 deflayer - no changes
                "(defsrc () 1  2)\
                 (deflayer base 0 1 2)",
                "(defsrc () 1  2)\
                 (deflayer base 0 1 2)",
            ),
            // (
            //     /*
            //     1 defsrc, 1 deflayer, but a line comment inside defsrc - formatting applied
            //     fixme: should the space before closing paren stay?
            //     */
            //     "(defsrc 1  2 # Mary had a little lamb\n)\
            //      (deflayer base 1 2)",
            //     "(defsrc 1  2 # Mary had a little lamb\n) \
            //      (deflayer base 1  2 )",
            // ),
            // (
            //     /*
            //     1 defsrc, 1 deflayer, but a line comment inside a deflayer - no formatting applied.
            //     fixme: investigate how this feels, and possibly change.
            //     */
            //     "(defsrc 1  2)\
            //      (deflayer base\n  1\n  # Mary had a little lamb\n  2)",
            //     "(defsrc 1  2)\
            //      (deflayer base 1 # Mary had a little lamb\n  2)",
            // ),
            // (
            //     // todo: same as the two above, but for block comments.
            //     "", "",
            // ),
        ];
        for (i, (case, expected_result)) in cases.iter().enumerate() {
            log!("===========================");
            log!("case {}", i);
            let mut tree = parse_into_ext_tree(case).expect("parses");
            tree.use_defsrc_layout_on_deflayers();
            assert_eq!(tree.to_string(), *expected_result);
        }
    }
}
