use crate::log;

use super::ext_tree::*;

impl ExtParseTree {
    // todo: maybe don't format if an atom in defsrc/deflayer is too large.
    pub fn use_defsrc_layout_on_deflayers<'a>(&'a mut self) {
        let mut defsrc: Option<&'a NodeList> = None;
        let mut deflayers: Vec<&'a mut NodeList> = vec![];

        for top_level_item in self.0.iter_mut() {
            let top_level_list = match &mut top_level_item.expr {
                Expr::Atom(_x) => continue,
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

        // Each item in layout is a string that consists only of whitespace characters.
        let mut layout: Vec<String> = vec![String::with_capacity(10); defsrc_item_count];

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

            for ch in defsrc_item_as_str.chars() {
                match ch {
                    '\n' => layout[i].push(ch),
                    '\t' => layout[i].push(ch),
                    _ => layout[i].push(' '),
                }
            }

            // NOTE: We intentionally process only `post_metadata` and ignore `pre_metadata`.
            // This should be either fixed later, or we just shouldn't modify `pre_metadata`
            // in previous operations on tree.
            for metadata in &defsrc_item.post_metadata {
                match metadata {
                    Metadata::LineComment(_) => {
                        log!("line comments unsupported for now");
                        return;
                    }
                    Metadata::BlockComment(_) => {
                        log!("block comments unsupported for now");
                        return;
                    }
                    Metadata::Whitespace(whitespace) => {
                        layout[i].push_str(whitespace);
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
                    "Formatting of '{}' deflayer skipped: items count doesn't match defsrc",
                    layer_name
                );
                continue;
            }

            for (i, deflayer_item) in deflayer.iter_mut().skip(2).enumerate() {
                let deflayer_item_as_str = deflayer_item.expr.to_string();

                let layout_size = layout[i].chars().count();
                let deflayer_item_size = deflayer_item_as_str.chars().count();

                // A hacky implementation for now. All comments will be deleted.
                if deflayer_item_size >= layout_size {
                    let mut iter = layout[i].chars();
                    for ch in iter.by_ref() {
                        if ch == '\n' {
                            let remainder = "\n".chars().chain(iter.by_ref()).collect();
                            deflayer_item
                                .post_metadata
                                .push(Metadata::Whitespace(remainder));
                            break;
                        }
                    }
                } else {
                    // for ch in layout[i].chars().skip(deflayer_item_size) { }
                    let ret = layout[i].chars().skip(deflayer_item_size).collect();
                    deflayer_item.post_metadata.clear();
                    deflayer_item.post_metadata.push(Metadata::Whitespace(ret));
                }
            }
        }
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
                // 1 defsrc, 2 deflayers - formatting applied for both deflayers
                "(defsrc \n 1  2\n) (deflayer base 1 2 ) ( deflayer\n\t layer2 \n\n3  \t  \n  \t4\n )",
                "(defsrc \n 1  2\n) (deflayer base 1  2\n) ( deflayer\n\t layer2 \n\n3  4\n)",
            ),
            (
                /*
                1 defsrc, 1 defalias, 2 deflayers - formatting applied for both deflayers,
                while defalias should be untouched.
                */
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
                /*
                1 defsrc, 1 deflayer, but the deflayer contains a multi-byte unicode
                character (but no multi-cluster) - formatting should be applied
                correctly regardless of used characters.
                */
                "(defsrc \n 🌍 1  2\n)  (deflayer base 🌍   1  \n 2 \t)",
                "(defsrc \n 🌍 1  2\n)  (deflayer base 🌍 1  2\n)",
            ),
            (
                /*
                1 defsrc, 1 deflayer, but the deflayer contains a multi-cluster
                unicode character - formatting should be applied correctly
                regardless of used characters.
                */
                "(defsrc \n 👨‍👩‍👧‍👦 1  2\n)  (deflayer base 👨‍👩‍👧‍👦 \t 1     2 \n\n)",
                "(defsrc \n 👨‍👩‍👧‍👦 1  2\n)  (deflayer base 👨‍👩‍👧‍👦 1  2\n)",
            ),
            (
                // 1 invalid defsrc, 1 deflayer - no changes
                "(defsrc () 1  2)  (deflayer base 0 1 2)",
                "(defsrc () 1  2)  (deflayer base 0 1 2)",
            ),
            // (
            //     /*
            //     1 defsrc, 1 deflayer, but a line comment inside defsrc - formatting applied
            //     fixme: should the space before closing paren stay?
            //     */
            //     "(defsrc 1  2 # Mary had a little lamb\n)  (deflayer base 1 2)",
            //     "(defsrc 1  2 # Mary had a little lamb\n)  (deflayer base 1  2 )",
            // ),
            // (
            //     /*
            //     1 defsrc, 1 deflayer, but a line comment inside a deflayer - no
            //     formatting applied.
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
            log!("({}) ===========================", i);
            let mut tree = parse_into_ext_tree(case).expect("parses");
            log!("case {}: {}", i, tree.to_string());
            tree.use_defsrc_layout_on_deflayers();
            assert_eq!(tree.to_string(), *expected_result);
        }
    }
}
