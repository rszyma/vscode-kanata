use std::borrow::BorrowMut;

use super::ext_tree::*;

impl ExtParseTree {
    /// Removes excessive adjacent newlines after `max` number of newlines.
    /// Depends on whitespace removal at ends of lines to work correctly.
    /// Otherwise it might look like it didn't remove some excessive newlines.
    pub fn remove_excessive_adjacent_newlines(&mut self, max: usize) {
        self.0.remove_excessive_adjacent_newlines(max);
    }
}

impl NodeList {
    fn remove_excessive_adjacent_newlines(&mut self, max: usize) {
        match self {
            NodeList::NonEmptyList(parse_trees) => {
                for tree in parse_trees {
                    tree.remove_excessive_adjacent_newlines(max);
                }
            }
            NodeList::EmptyList(metadatas) => {
                for metadata in metadatas {
                    metadata.remove_excessive_adjacent_newlines(max);
                }
            }
        }
    }
}

impl ParseTreeNode {
    fn remove_excessive_adjacent_newlines(&mut self, max: usize) {
        for metadata in &mut self.pre_metadata {
            metadata.remove_excessive_adjacent_newlines(max);
        }
        if let Expr::List(list) = &mut self.expr {
            list.remove_excessive_adjacent_newlines(max)
        }
        for metadata in &mut self.post_metadata {
            metadata.remove_excessive_adjacent_newlines(max);
        }
    }
}

impl Metadata {
    fn remove_excessive_adjacent_newlines(&mut self, max: usize) {
        if let Metadata::Whitespace(x) = self.borrow_mut() {
            loop {
                let mut consecutive_newlines: usize = 0;
                let mut replace_at_index: Option<usize> = None;
                for (i, char) in x.chars().enumerate() {
                    if char == '\n' {
                        consecutive_newlines += 1;

                        if consecutive_newlines > max {
                            replace_at_index = Some(i);
                        }
                    } else {
                        if consecutive_newlines > max {
                            break;
                        }
                        consecutive_newlines = 0;
                    }
                }
                if let Some(i) = replace_at_index {
                    let newlines_to_remove = consecutive_newlines - max;
                    x.drain((i + 1 - newlines_to_remove)..(i + 1));
                    continue;
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::log;

    use super::*;

    #[test]
    fn test_remove_above_2_adjacent_newlines() {
        #[rustfmt::skip]
        let cases = vec![
            (
                1,
                "(1 \n\n\n 2)",
                "(1 \n 2)"
            ),
            (
                0,
                "(1 \n\n\n 2)",
                "(1  2)"
            ),
            (
                1,
                "\n\n\n\n",
                "\n"
            ),
            (
                0,
                "\n\n\n\n",
                ""
            ),
            (
                3,
                "\n\n\n\n\n",
                "\n\n\n"
            ),
            (
                3,
                "\n\n\n\n\n \n\n\n\n \n\n",
                "\n\n\n \n\n\n \n\n"
            ),
            (
                2,
                "(\n(\n\n\n (1\n \n\n\n\n ))\n)\n \n(\n\n\n 2\n)\n",
                "(\n(\n\n (1\n \n\n ))\n)\n \n(\n\n 2\n)\n",
            ),
        ];
        for (max, case, expected_result) in cases {
            log!("==============");
            log!("max: {:?}", max);
            let mut tree = parse_into_ext_tree(case).expect("parses");
            assert_eq!(tree.to_string(), case);
            tree.remove_excessive_adjacent_newlines(max);
            assert_eq!(tree.to_string(), expected_result);
        }
    }
}
