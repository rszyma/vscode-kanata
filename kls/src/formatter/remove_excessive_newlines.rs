use std::borrow::BorrowMut;

use super::ext_tree::*;

impl ParseTreeNode {
    /// Removes excessive adjacent newlines after `max` number of newlines.
    /// Depends on whitespace removal at ends of lines to work correctly.
    /// Otherwise it might look like it didn't remove some excessive newlines.
    pub fn remove_excessive_adjacent_newlines(&mut self, max: usize) {
        match self.borrow_mut() {
            ParseTreeNode::List(list) => {
                for node in list {
                    node.remove_excessive_adjacent_newlines(max);
                }
            }
            ParseTreeNode::Whitespace(x) => loop {
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
            },
            _ => {}
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
            tree.0.remove_excessive_adjacent_newlines(max);
            assert_eq!(tree.to_string(), expected_result);
        }
    }
}
