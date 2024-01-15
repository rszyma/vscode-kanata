use kanata_parser::cfg::{
    sexpr::{self, Position, SExpr, SExprMetaData, Span, Spanned},
    ParseError,
};
use std::fmt::{Debug, Display};

use crate::{helpers, log};

#[derive(Debug, PartialEq, Eq)]
pub struct ExtParseTree(pub NodeList);

impl ExtParseTree {
    /// Appends Expr to the last node at given `depth`, which is expected
    /// to be a list (otherwise panics).
    pub fn push_expr(&mut self, depth: usize, expr: Expr) {
        let mut head: &mut NodeList = &mut self.0;
        for _ in 0..depth {
            let node = match head.last_mut() {
                Some(x) => x,
                None => panic!("depth out-of bounds"),
            };
            match &mut node.expr {
                Expr::Atom(_) => {
                    panic!("last item is not a list!")
                }
                Expr::List(xs) => {
                    head = xs;
                }
            }
        }
        head.push(ParseTreeNode::without_metadata(expr));
    }

    /// Appends Metadata to the last node at given `depth`, which is
    /// expected to be a list (otherwise panics).
    pub fn push_metadata(&mut self, depth: usize, metadata: Metadata) {
        let mut head: &mut NodeList = &mut self.0;
        for _ in 0..depth {
            let node = match head.last_mut() {
                Some(x) => x,
                None => panic!("depth out-of bounds"),
            };
            match &mut node.expr {
                Expr::Atom(_) => {
                    panic!("last item is not a list!")
                }
                Expr::List(xs) => {
                    head = xs;
                }
            }
        }
        match head {
            NodeList::NonEmptyList(xs) => xs.last_mut().unwrap().post_metadata.push(metadata),
            NodeList::EmptyList(metadatas) => {
                metadatas.push(metadata);
            }
        }
    }
}

impl ExtParseTree {
    // If any step on path is not List, panic.
    // If any step is out-of-bounds, return None.
    // pub fn get_node(&self, at_path: &[usize]) -> Option<&ParseTreeNode> {
    //     if at_path.is_empty() {
    //         return None;
    //     }
    //     return match self.0.get(at_path[0]) {
    //         Some(x) => x,
    //         None => return None,
    //     }
    //     .get_node(&at_path[1..]);
    // }

    // pub fn get_node_mut(&mut self, at_path: &[usize]) -> Option<&mut ParseTreeNode> {
    //     if at_path.is_empty() {
    //         return None;
    //     }
    //     return match self.0.get(at_path[0]) {
    //         Some(x) => x,
    //         None => return None,
    //     }
    //     .get_node_mut(&at_path[1..]);
    // }
}

impl Display for ExtParseTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comment {
    LineComment(String),
    BlockComment(String),
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Comment::LineComment(x) => write!(f, "{}", x)?,
            Comment::BlockComment(x) => write!(f, "{}", x)?,
        };
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Metadata {
    Comment(Comment),
    Whitespace(String),
}

impl Metadata {
    pub fn value(&self) -> &str {
        match self {
            Metadata::Comment(c) => match c {
                Comment::LineComment(x) => x,
                Comment::BlockComment(x) => x,
            },
            Metadata::Whitespace(x) => x,
        }
    }
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Metadata::Comment(x) => write!(f, "{}", x)?,
            Metadata::Whitespace(x) => write!(f, "{}", x)?,
        };
        Ok(())
    }
}

impl From<SExprMetaData> for Metadata {
    fn from(value: SExprMetaData) -> Self {
        match value {
            SExprMetaData::LineComment(x) => Metadata::Comment(Comment::LineComment(x.t)),
            SExprMetaData::BlockComment(x) => Metadata::Comment(Comment::BlockComment(x.t)),
            SExprMetaData::Whitespace(x) => Metadata::Whitespace(x.t),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeList {
    NonEmptyList(Vec<ParseTreeNode>),
    EmptyList(Vec<Metadata>),
}

impl NodeList {
    pub fn len(&self) -> usize {
        match self {
            NodeList::NonEmptyList(xs) => xs.len(),
            NodeList::EmptyList(_) => 0,
        }
    }

    pub fn push(&mut self, mut node: ParseTreeNode) {
        let metadata = match self {
            NodeList::NonEmptyList(xs) => {
                xs.push(node);
                return;
            }
            NodeList::EmptyList(xs) => xs,
        };
        metadata.append(&mut node.pre_metadata);
        node.pre_metadata = metadata.to_vec();
        *self = NodeList::NonEmptyList(vec![node]);
    }

    pub fn last(&self) -> Option<&ParseTreeNode> {
        match self {
            NodeList::NonEmptyList(xs) => xs.last(),
            NodeList::EmptyList(_) => None,
        }
    }

    pub fn last_mut(&mut self) -> Option<&mut ParseTreeNode> {
        match self {
            NodeList::NonEmptyList(xs) => xs.last_mut(),
            NodeList::EmptyList(_) => None,
        }
    }

    pub fn get(&self, index: usize) -> Option<&ParseTreeNode> {
        match self {
            NodeList::NonEmptyList(xs) => xs.get(index),
            NodeList::EmptyList(_) => None,
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut ParseTreeNode> {
        match self {
            NodeList::NonEmptyList(xs) => xs.get_mut(index),
            NodeList::EmptyList(_) => None,
        }
    }

    // Function to create iterator over ParseTreeNode elements in List enum
    pub fn iter(&self) -> impl Iterator<Item = &'_ ParseTreeNode> {
        match self {
            NodeList::NonEmptyList(nodes) => nodes.iter(),
            NodeList::EmptyList(_) => [].iter(), // Return an empty iterator for EmptyList
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut ParseTreeNode> {
        match self {
            NodeList::NonEmptyList(nodes) => nodes.iter_mut(),
            NodeList::EmptyList(_) => [].iter_mut(), // Return an empty mutable iterator for EmptyList
        }
    }
}

impl Default for NodeList {
    fn default() -> Self {
        NodeList::EmptyList(vec![])
    }
}

impl Display for NodeList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeList::NonEmptyList(xs) => {
                for x in xs.iter() {
                    write!(f, "{}", x)?;
                }
            }
            NodeList::EmptyList(xs) => {
                for x in xs {
                    write!(f, "{}", x)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
    Atom(String),
    List(NodeList),
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Atom(x) => {
                write!(f, "{}", x)?;
            }
            Expr::List(list) => {
                write!(f, "({})", list)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseTreeNode {
    pub pre_metadata: Vec<Metadata>,
    pub expr: Expr,
    pub post_metadata: Vec<Metadata>,
}

impl ParseTreeNode {
    pub fn without_metadata(expr: Expr) -> ParseTreeNode {
        ParseTreeNode {
            pre_metadata: vec![],
            expr,
            post_metadata: vec![],
        }
    }
}

impl ParseTreeNode {
    // If any step on path is not List, panic.
    // If any step is out-of-bounds, return None.
    // pub fn get_node(&self, at_path: &[usize]) -> Option<&ParseTreeNode> {
    //     let mut head: &ParseTreeNode = self;
    //     for i in at_path {
    //         if let ParseTreeNode::List(l) = head {
    //             head = match l.get(*i) {
    //                 Some(x) => x,
    //                 None => return None,
    //             };
    //         } else {
    //             panic!("invalid tree path")
    //         }
    //     }
    //     Some(head)
    // }

    // pub fn get_node_mut(&mut self, at_path: &[usize]) -> Option<&mut ParseTreeNode> {
    //     let mut head: &mut ParseTreeNode = self;
    //     for i in at_path {
    //         if let ParseTreeNode::List(l) = head {
    //             head = match l.get_mut(*i) {
    //                 Some(x) => x,
    //                 None => return None,
    //             };
    //         } else {
    //             panic!("invalid tree path")
    //         }
    //     }
    //     Some(head)
    // }

    // Panics if the variant is not List.
    // pub fn unwrap_list(&self) -> &NodeList {
    //     match &self.expr {
    //         Expr::List(list) => list,
    //         _ => panic!("not a list"),
    //     }
    // }

    // // Panics if the variant is not List.
    // pub fn unwrap_list_mut(&mut self) -> &mut NodeList {
    //     match &mut self.expr {
    //         Expr::List(list) => list,
    //         _ => panic!("not a list"),
    //     }
    // }
}

impl Display for ParseTreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ParseTreeNode {
            pre_metadata,
            expr,
            post_metadata,
        } = self;

        for metadata in pre_metadata {
            write!(f, "{}", metadata)?;
        }

        write!(f, "{}", expr)?;

        for metadata in post_metadata {
            write!(f, "{}", metadata)?;
        }

        Ok(())
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
    let start_time = helpers::now();
    let filename = "";
    let (exprs, exprs_ext) = sexpr::parse_(src, filename, false)?;
    let exprs: Vec<SExpr> = exprs.into_iter().map(SExpr::List).collect();
    let exprs_len = exprs.len();
    let last_sexpr_end = exprs.last().map(|x| x.span().end).unwrap_or_default();
    let last_metadata_end = exprs_ext.last().map(|x| x.span().end).unwrap_or_default();
    let root_span = CustomSpan {
        start: Position::default(),
        end: {
            if last_sexpr_end.absolute > last_metadata_end.absolute {
                last_sexpr_end
            } else {
                last_metadata_end
            }
        },
        file_name: filename.to_string(),
        file_content: src,
    };
    let exprs = {
        let mut r = SExpr::List(Spanned::new(
            Vec::with_capacity(exprs_len),
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
    let mut tree: ExtParseTree = ExtParseTree(NodeList::default());
    let mut tree_depth: usize = 0; // currentdepth of the list we're currently appending to in `tree`.
    let mut expr_path: Vec<usize> = vec![0]; // path to the current item in `exprs` tree.
    loop {
        match exprs.get_node(&expr_path) {
            Some(expr) => {
                while let Some(metadata) =
                    metadata_iter.next_if(|m| m.span().start() < expr.span().start())
                {
                    tree.push_metadata(tree_depth, metadata.into());
                }

                match expr {
                    SExpr::Atom(x) => {
                        tree.push_expr(tree_depth, Expr::Atom(x.t.clone()));
                        match expr_path.last_mut() {
                            Some(i) => *i += 1,
                            None => unreachable!(),
                        };
                    }
                    SExpr::List(_) => {
                        tree.push_expr(tree_depth, Expr::List(NodeList::default()));
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
                    tree.push_metadata(tree_depth, metadata.into());
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
        tree.push_metadata(tree_depth, metadata.into());
    }

    log!(
        "parse_into_ext_tree_and_root_span in {:.3?}",
        helpers::now().duration_since(start_time)
    );

    Ok((tree, root_span))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log;

    macro_rules! Tree {
        ($($element:expr),*) => {{
            ExtParseTree(NodeList::NonEmptyList(
                vec![$($element),*]
            ))
        }};
    }

    macro_rules! Atom {
        ($text:expr) => {
            Expr::Atom($text.to_string())
        };
    }

    macro_rules! LineComment {
        ($text:expr) => {
            Metadata::Comment(Comment::LineComment($text.to_string()))
        };
    }

    macro_rules! BlockComment {
        ($text:expr) => {
            Metadata::Comment(Comment::BlockComment($text.to_string()))
        };
    }

    macro_rules! Whitespace {
        ($text:expr) => {
            Metadata::Whitespace($text.to_string())
        };
    }

    macro_rules! NonEmptyList {
        ($($element:expr),*) => {{
            Expr::List(NodeList::NonEmptyList(vec![$($element),*]))
        }};
    }

    macro_rules! EmptyList {
        ($($element:expr),*) => {{
            Expr::List(NodeList::EmptyList(vec![$($element),*]))
        }};
    }

    #[test]
    fn test_macros() {
        // use ParseTreeNode::*;
        // assert_eq!(Tree!(), ExtParseTree(NodeList(vec![])));

        // assert_eq!(
        //     Tree!(List!()),
        //     ExtParseTree(NodeList(vec![NodeList(vec![])]))
        // );

        // assert_eq!(
        //     Tree!(
        //         Atom!("test"),
        //         Whitespace!(" "),
        //         LineComment!("# test"),
        //         BlockComment!("#| test |#")
        //     ),
        //     ExtParseTree(NodeList(vec![
        //         Atom("test".to_string()),
        //         Whitespace(" ".to_string()),
        //         LineComment("# test".to_string()),
        //         BlockComment("#| test |#".to_string())
        //     ]))
        // );
    }

    #[test]
    fn test_parse_into_ext_tree() {
        #[rustfmt::skip]
        let cases = vec![
            (
                "",
                ExtParseTree(NodeList::default())
            ),
            (
                "\n",
                ExtParseTree(NodeList::EmptyList(vec![Whitespace!("\n")]))
            ),
            (
                "()",
                Tree!(
                    ParseTreeNode::without_metadata(EmptyList!())
                )
            ),
            (
                "(atom)",
                Tree!(
                    ParseTreeNode::without_metadata(NonEmptyList!(
                        ParseTreeNode::without_metadata(Atom!("atom"))
                    ))
                )
            ),
            (
                "( test)(1 \n\t 2)",
                Tree!(
                    ParseTreeNode::without_metadata(NonEmptyList!(
                        ParseTreeNode{
                            pre_metadata: vec![Whitespace!(" ")],
                            expr: Atom!("test"),
                            post_metadata: vec![]
                        }
                    )),
                    ParseTreeNode::without_metadata(NonEmptyList!(
                        ParseTreeNode{
                            pre_metadata: vec![],
                            expr: Atom!("1"),
                            post_metadata: vec![Whitespace!(" \n\t ")],
                        },
                        ParseTreeNode::without_metadata(Atom!("2"))
                    ))
                )
            ),
            (
                "(1 2 #|block|# 3)",
                Tree!(
                    ParseTreeNode::without_metadata(NonEmptyList!(
                        ParseTreeNode{
                            pre_metadata: vec![],
                            expr: Atom!("1"),
                            post_metadata: vec![Whitespace!(" ")]
                        },
                        ParseTreeNode{
                            pre_metadata: vec![],
                            expr: Atom!("2"),
                            post_metadata: vec![
                                Whitespace!(" "),
                                BlockComment!("#|block|#"),
                                Whitespace!(" "),
                            ],
                        },
                        ParseTreeNode::without_metadata(Atom!("3"))
                    ))
                )
            ),
            (
                "(1\n)",
                Tree!(
                    ParseTreeNode::without_metadata(NonEmptyList!(
                        ParseTreeNode{
                            pre_metadata: vec![],
                            expr: Atom!("1"),
                            post_metadata: vec![Whitespace!("\n")]
                        }
                    ))
                )
            ),
            (
                "\n(1\n) \n ;; comment \n\t (2) ",
                (
                    Tree!(
                        ParseTreeNode{
                            pre_metadata: vec![Whitespace!("\n")],
                            expr: NonEmptyList!(
                                ParseTreeNode{
                                    pre_metadata: vec![],
                                    expr: Atom!("1"),
                                    post_metadata: vec![Whitespace!("\n")]
                                }
                            ),
                            post_metadata: vec![
                                Whitespace!(" \n "),
                                LineComment!(";; comment \n"),
                                Whitespace!("\t ")
                            ]
                        },
                        ParseTreeNode{
                            pre_metadata: vec![],
                            expr: NonEmptyList!(
                                ParseTreeNode::without_metadata(Atom!("2"))
                            ),
                            post_metadata: vec![Whitespace!(" ")]
                        }
                    )
                ),
            ),
            (   // comment at the end of a file
                "(123)\n;;",
                Tree!(
                    ParseTreeNode{
                        pre_metadata: vec![],
                        expr: NonEmptyList!(
                            ParseTreeNode::without_metadata(Atom!("123"))
                        ),
                        post_metadata: vec![
                            Whitespace!("\n"),
                            LineComment!(";;")
                        ]
                    }
                )
            ),
        ];
        for (case, expected_result) in cases {
            log!("===========================");
            let actual_result = parse_into_ext_tree(case).expect("parses");
            log!("actual: {:#?}", actual_result);
            log!("expected: {:#?}", expected_result);
            assert_eq!(actual_result, expected_result, "parse_into_ext_tree(case)");
            log!("actual (string): {:#?}", actual_result.to_string());
            assert_eq!(
                actual_result.to_string(),
                case,
                "<ExtParseTree>.to_string()"
            );
        }
    }
}
