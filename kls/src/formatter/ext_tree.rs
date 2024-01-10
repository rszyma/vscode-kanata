use kanata_parser::cfg::{
    sexpr::{self, Position, SExpr, SExprMetaData, Span, Spanned},
    ParseError,
};
use std::{borrow::BorrowMut, fmt::Display};

/// Extended Parse Tree.
/// Let's represent the whole config as a [`ParseTreeNode`].
/// Root node should be [`ParseTreeNode::List`] containing other top level items.
/// While [`sexpr::parse_`] can only return [`ParseTreeNode::List`]s as top-level items,
/// this doesn't change anything in regard to how we use this struct later.
#[derive(PartialEq, Eq, Debug)]
pub struct ExtParseTree(pub ParseTreeNode);

#[derive(PartialEq, Eq, Debug)]
pub enum ParseTreeNode {
    List(Vec<ParseTreeNode>),

    Atom(String),
    LineComment(String),
    BlockComment(String),
    /// Besides normal use of `Whitespace` items, additional "empty" `Whitespace`s are
    /// generated before a `List` node and as a first node inside `List`s (except root list),
    /// and they are generated, so formatter can run more efficiently.
    Whitespace(String),
}

impl ParseTreeNode {
    pub fn width(&self) -> usize {
        match self {
            Self::List(xs) => xs.iter().fold(2, |acc, node| acc + node.width()),
            Self::Atom(x) | Self::BlockComment(x) | Self::Whitespace(x) => x.chars().count(),
            Self::LineComment(x) => x.chars().count() + 1,
        }
    }
}

/// Parses config from text and combines both [`SExpr`] and [`SExprMetaData`] into [`ExtParseTree`].
pub fn parse_into_ext_tree(src: &str) -> std::result::Result<ExtParseTree, ParseError> {
    parse_into_ext_tree_and_root_span(src).map(|(x1, _)| x1)
}

/// Compared to [`kanata_parser::cfg::sexpr::Span`], this struct uses `&'a str`
/// for `file_content` and String for `file_name` instead of using
/// [`alloc::rc::Rc<str>`] for both.
#[derive(Clone)]
pub struct CustomSpan<'a> {
    pub start: Position,
    pub end: Position,
    pub file_name: String,
    pub file_content: &'a str,
}

impl<'a> From<CustomSpan<'a>> for Span {
    fn from(val: CustomSpan<'a>) -> Self {
        Span {
            start: val.start,
            end: val.end,
            file_name: val.file_name.into(),
            file_content: val.file_content.into(),
        }
    }
}

pub fn parse_into_ext_tree_and_root_span(
    src: &str,
) -> std::result::Result<(ExtParseTree, CustomSpan<'_>), ParseError> {
    let filename = "";
    let (exprs, exprs_ext) = sexpr::parse_(src, filename, false)?;
    let exprs: Vec<SExpr> = exprs.into_iter().map(SExpr::List).collect();
    let position_end = exprs.last().map(|x| x.span().end).unwrap_or_default();
    let root_span = CustomSpan {
        start: Position::default(),
        end: position_end,
        file_name: filename.to_string(),
        file_content: src,
    };
    let exprs = {
        let mut r = SExpr::List(Spanned::new(
            Vec::with_capacity(exprs.len()),
            root_span.clone().into(),
        ));
        for x in exprs {
            if let SExpr::List(Spanned { t, .. }) = &mut r {
                t.push(x)
            }
        }
        r
    };
    let exprs: SExprCustom = SExprCustom(exprs);
    // crate::log!("sexprs: {:?}", sexprs);

    let mut metadata_iter = exprs_ext.into_iter().peekable();
    let mut tree: ExtParseTree = ExtParseTree(ParseTreeNode::List(vec![]));
    let mut tree_depth: u8 = 0; // currentdepth of the list we're currently appending to in `tree`.
    let mut expr_path: Vec<usize> = vec![0]; // path to the current item in `exprs` tree.
    loop {
        match exprs.get_node(&expr_path) {
            Some(expr) => {
                while let Some(metadata) =
                    metadata_iter.next_if(|m| m.span().start() < expr.span().start())
                {
                    tree.append(
                        tree_depth,
                        match metadata {
                            SExprMetaData::LineComment(m) => ParseTreeNode::LineComment(m.t),
                            SExprMetaData::BlockComment(m) => ParseTreeNode::BlockComment(m.t),
                            SExprMetaData::Whitespace(m) => ParseTreeNode::Whitespace(m.t),
                        },
                    )
                }

                match expr {
                    SExpr::Atom(x) => {
                        tree.append(tree_depth, ParseTreeNode::Atom(x.t.clone()));
                        match expr_path.last_mut() {
                            Some(i) => *i += 1,
                            None => unreachable!(),
                        };
                    }
                    SExpr::List(_) => {
                        // Push empty Whitespace before a new List and another one
                        // as the first list item, as an optitmization to formatter.
                        // tree.append(tree_depth, ParseTreeNode::Whitespace("".to_string()));
                        tree.append(
                            tree_depth,
                            ParseTreeNode::List(vec![
                                // ParseTreeNode::Whitespace("".to_string())
                            ]),
                        );
                        tree_depth += 1;
                        expr_path.push(0);
                    }
                }
            }
            None => {
                // Reached the end of the list.
                expr_path.pop();

                // Get the absolute position of the closing paren, and push all leftover metadata
                // that's located before it.
                let expr = exprs
                    .get_node(&expr_path)
                    .expect("should exist, we just iterated over it");
                while let Some(metadata) =
                    metadata_iter.next_if(|m| m.span().start() < expr.span().end())
                {
                    tree.append(
                        tree_depth,
                        match metadata {
                            SExprMetaData::LineComment(m) => ParseTreeNode::LineComment(m.t),
                            SExprMetaData::BlockComment(m) => ParseTreeNode::BlockComment(m.t),
                            SExprMetaData::Whitespace(m) => ParseTreeNode::Whitespace(m.t),
                        },
                    )
                }

                match expr_path.last_mut() {
                    Some(i) => *i += 1,
                    None => break,
                };
                tree_depth -= 1;
            }
        };
    }

    // Add remaining metadata.
    for metadata in metadata_iter {
        tree.append(
            tree_depth,
            match metadata {
                SExprMetaData::LineComment(m) => ParseTreeNode::LineComment(m.t),
                SExprMetaData::BlockComment(m) => ParseTreeNode::BlockComment(m.t),
                SExprMetaData::Whitespace(m) => ParseTreeNode::Whitespace(m.t),
            },
        )
    }

    Ok((tree, root_span))
}

impl ParseTreeNode {
    /// If any step on path is not List, panic.
    /// If any step is out-of-bounds, return None.
    pub fn get_node(&self, at_path: &[usize]) -> Option<&ParseTreeNode> {
        let mut head: &ParseTreeNode = self;
        for i in at_path {
            if let ParseTreeNode::List(l) = head {
                head = match l.get(*i) {
                    Some(x) => x,
                    None => return None,
                };
            } else {
                panic!("invalid tree path")
            }
        }
        Some(head)
    }

    pub fn get_node_mut(&mut self, at_path: &[usize]) -> Option<&mut ParseTreeNode> {
        let mut head: &mut ParseTreeNode = self;
        for i in at_path {
            if let ParseTreeNode::List(l) = head {
                head = match l.get_mut(*i) {
                    Some(x) => x,
                    None => return None,
                };
            } else {
                panic!("invalid tree path")
            }
        }
        Some(head)
    }

    pub fn unwrap_list(&self) -> &Vec<Self> {
        match self {
            Self::List(list) => list,
            _ => panic!("not a list"),
        }
    }

    // Panics if the variant is not List.
    pub fn unwrap_list_mut(&mut self) -> &mut Vec<Self> {
        match self {
            Self::List(list) => list,
            _ => panic!("not a list"),
        }
    }
}

impl ExtParseTree {
    /// Appends `node` to the last node at given `depth`, which is expected
    /// to be a list (otherwise panics).
    fn append(&mut self, depth: u8, node: ParseTreeNode) {
        let mut head: &mut ParseTreeNode = self.0.borrow_mut();
        for _ in 0..depth {
            if let ParseTreeNode::List(li) = head {
                head = li.last_mut().expect("path is valid");
            } else {
                panic!("unexpected non-list item")
            }
        }
        if let ParseTreeNode::List(ref mut l) = head {
            l.push(node);
        } else {
            panic!("unexpected non-list item");
        }
    }

    /// If any step on path is not List, panic.
    /// If any step is out-of-bounds, return None.
    pub fn get_node(&self, at_path: &[usize]) -> Option<&ParseTreeNode> {
        return self.0.get_node(at_path);
    }

    pub fn get_node_mut(&mut self, at_path: &[usize]) -> Option<&mut ParseTreeNode> {
        return self.0.get_node_mut(at_path);
    }

    fn width(&self) -> usize {
        self.unwrap_list()
            .iter()
            .fold(0, |acc, node| acc + node.width())
    }

    pub fn unwrap_list(&self) -> &Vec<ParseTreeNode> {
        match &self.0 {
            ParseTreeNode::List(list) => list,
            _ => panic!("not a list"),
        }
    }

    pub fn unwrap_list_mut(&mut self) -> &mut Vec<ParseTreeNode> {
        match &mut self.0 {
            ParseTreeNode::List(list) => list,
            _ => panic!("not a list"),
        }
    }
}

struct SExprCustom(SExpr);

impl<'a> SExprCustom {
    /// If any step on path is not List, panic.
    /// If any step is out-of-bounds, return None.
    fn get_node(&'a self, at_path: &[usize]) -> Option<&'a SExpr> {
        let mut head: &SExpr = &self.0; // is it ok to do this?
        for i in at_path {
            if let SExpr::List(Spanned { t, .. }) = head {
                head = match &t.get(*i) {
                    Some(x) => x,
                    None => return None,
                };
            } else {
                panic!("invalid tree path")
            }
        }
        Some(head)
    }
}

impl Display for ExtParseTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let ParseTreeNode::List(l) = &self.0 {
            for expr in l {
                write!(f, "{}", expr)?;
            }
        } else {
            // root node must be a list.
            return Err(std::fmt::Error);
        }
        Ok(())
    }
}

impl Display for ParseTreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseTreeNode::List(l) => {
                write!(f, "(")?;
                for expr in l {
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")?;
            }
            ParseTreeNode::Atom(x)
            | ParseTreeNode::LineComment(x)
            | ParseTreeNode::BlockComment(x)
            | ParseTreeNode::Whitespace(x) => {
                write!(f, "{}", x)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log;

    macro_rules! Tree {
        ($($element:expr),*) => {{
            ExtParseTree(ParseTreeNode::List(vec![$($element),*]))
        }};
    }

    macro_rules! Atom {
        ($text:expr) => {
            ParseTreeNode::Atom($text.to_string())
        };
    }

    macro_rules! LineComment {
        ($text:expr) => {
            ParseTreeNode::LineComment($text.to_string())
        };
    }

    macro_rules! BlockComment {
        ($text:expr) => {
            ParseTreeNode::BlockComment($text.to_string())
        };
    }

    macro_rules! Whitespace {
        ($text:expr) => {
            ParseTreeNode::Whitespace($text.to_string())
        };
    }

    macro_rules! List {
        ($($element:expr),*) => {{
            ParseTreeNode::List(vec![ParseTreeNode::Whitespace("".to_string()), $($element),*])
        }};
    }

    #[test]
    fn test_macros() {
        use ParseTreeNode::*;
        assert_eq!(Tree!(), ExtParseTree(List(vec![])));

        #[rustfmt::skip]
        assert_eq!(
            Tree!(
                List!()
            ),
            ExtParseTree(List(vec![
                List(vec![
                    Whitespace("".to_string())
                ])
            ]))
        );

        assert_eq!(
            Tree!(
                Atom!("test"),
                Whitespace!(" "),
                LineComment!("# test"),
                BlockComment!("#| test |#")
            ),
            ExtParseTree(List(vec![
                Atom("test".to_string()),
                Whitespace(" ".to_string()),
                LineComment("# test".to_string()),
                BlockComment("#| test |#".to_string())
            ]))
        );
    }

    #[test]
    fn test_parse_into_ext_tree() {
        let s = "";
        assert_eq!(parse_into_ext_tree(s).expect("parses"), Tree!());

        #[rustfmt::skip]
        let cases = vec![
            (
                "",
                Tree!()
            ),
            (
                "()",
                Tree!(
                    Whitespace!(""),
                    List!()
                )
            ),
            (
                "(atom)",
                Tree!(
                    Whitespace!(""),
                    List!(Atom!("atom"))
                )
            ),
            (
                "( test)(1 \n\t 2)",
                Tree!(
                    Whitespace!(""),
                    List!(
                        Whitespace!(" "),
                        Atom!("test")
                    ),
                    Whitespace!(""),
                    List!(
                        Atom!("1"),
                        Whitespace!(" \n\t "),
                        Atom!("2")
                    )
                )
            ),
            (
                " \n\t \n",
                Tree!(Whitespace!(" \n\t \n"))
            ),
            (
                "(1 2 #|block|# 3)",
                Tree!(
                    Whitespace!(""),
                    List!(
                        Atom!("1"),
                        Whitespace!(" "),
                        Atom!("2"),
                        Whitespace!(" "),
                        BlockComment!("#|block|#"),
                        Whitespace!(" "),
                        Atom!("3")
                    )
                )
            ),
            (
                "(1\n)",
                Tree!(
                    Whitespace!(""),
                    List!(
                        Atom!("1"),
                        Whitespace!("\n")
                    )
                )
            ),
            (
                "\n(1\n) \n ;; comment \n\t (2) ",
                Tree!(
                    Whitespace!("\n"),
                    Whitespace!(""),
                    List!(
                        Atom!("1"),
                        Whitespace!("\n")
                    ),
                    Whitespace!(" \n "),
                    LineComment!(";; comment \n"),
                    Whitespace!("\t "),
                    Whitespace!(""),
                    List!(
                        Atom!("2")
                    ),
                    Whitespace!(" ")
                )
            ),
        ];
        for (case, expected_result) in cases {
            log!("===========================");
            let actual_result = parse_into_ext_tree(case).expect("parses");
            log!("actual: {:#?}", actual_result);
            log!("expected: {:#?}", expected_result);
            assert_eq!(actual_result, expected_result, "parse_into_ext_tree(case)");
            assert_eq!(
                actual_result.to_string(),
                case,
                "<ExtParseTree>.to_string()"
            );
        }
    }

    #[test]
    fn test_node_width() {
        let cases = vec![
            ("", 0),
            ("\n", 1),
            ("( )", 3),
            ("(23456789)", 10),
            ("(()())", 6),
            ("((34)(78))", 10),
            (";;bird", 7), // 2 for comment token + 4 for "bird" + 1 for newline
            ("#|hello|#", 9),
        ];
        for (case, expected_result) in cases {
            log!("===========================");
            log!("case: {}", case);
            let tree = parse_into_ext_tree(case).expect("parses");
            let actual_result = tree.width();
            assert_eq!(actual_result, expected_result, "tree node span");
        }
    }
}
