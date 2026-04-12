use anyhow::anyhow;
use itertools::Itertools;
use kanata_parser::cfg::{
    sexpr::{self, Position, SExpr, SExprMetaData, Span, Spanned},
    ParseError,
};
use std::{fmt::Display, path::PathBuf, str::FromStr};

/// ExtParseTree exists to allow efficient modification of nodes, with intention of combining back
/// later to the original source form, easily done by just calling .string() on it.
///
/// One downside of this form, is that nodes don't hold span/position info. So random access
/// by position will be at best O(n).
#[derive(Debug, PartialEq, Eq, Clone)]
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

    pub fn _get_node_by_path(&self, path: &[usize]) -> anyhow::Result<&Expr> {
        let mut head: &NodeList = &self.0;
        let last_path_index = path.len() - 1;
        for (path_index, &i) in path.iter().enumerate() {
            let node = match head.get(i) {
                Some(x) => x,
                None => panic!("path out-of-bounds on path index {path_index} "),
            };
            if path_index == last_path_index {
                return Ok(&node.expr);
            }
            match &node.expr {
                Expr::Atom(_) => {
                    return Err(anyhow!(
                        "atom found in the middle of path, while it's only allowed at the end"
                    ));
                }
                Expr::List(xs) => {
                    head = xs;
                }
            }
        }
        unreachable!()
    }

    pub fn includes(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut result = vec![];
        for top_level_block in self.0.iter() {
            if let Expr::List(NodeList::NonEmptyList(xs)) = &top_level_block.expr {
                match &xs[0].expr {
                    Expr::Atom(x) => match x.as_str() {
                        "include" => {}
                        _ => continue,
                    },
                    _ => continue,
                };

                if xs.len() != 2 {
                    return Err(anyhow!(
                        "an include block items: 2 != {}; block: \n{}",
                        xs.len(),
                        xs.iter().fold(String::new(), |mut acc, x| {
                            acc.push_str(&x.to_string());
                            acc
                        })
                    ));
                }

                if let Expr::Atom(x) = &xs[1].expr {
                    result.push(PathBuf::from_str(x.as_str().trim_matches('\"'))?)
                }
            };
        }
        Ok(result)
    }

    pub fn path_to_node_by_lsp_position(
        &self,
        pos: lsp_types::Position,
    ) -> anyhow::Result<Vec<u32>> {
        self.0.path_to_node_by_lsp_pos(pos, &mut 0, &mut 0)
    }
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

    #[allow(unused)]
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

    #[allow(unused)]
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

    fn path_to_node_by_lsp_pos(
        &self,
        pos: lsp_types::Position,
        line: &mut u32,
        chars_since_newline: &mut u32,
    ) -> Result<Vec<u32>, anyhow::Error> {
        for (i, current_node) in self.iter().enumerate() {
            let ParseTreeNode {
                pre_metadata,
                expr,
                post_metadata,
            } = current_node;

            for m in pre_metadata {
                *line += m.to_string().encode_utf16().fold(0, |acc, n| {
                    if n == b'\n' as u16 {
                        *chars_since_newline = 0;
                        acc + 1
                    } else {
                        *chars_since_newline += 1;
                        acc
                    }
                });
            }
            if *line > pos.line {
                return Err(anyhow!("position is inside metadata (1)"));
            }

            match expr {
                Expr::Atom(atom) => {
                    let len: u32 = atom.encode_utf16().collect_vec().len() as u32;
                    if *line == pos.line {
                        let at_least_lower_bound = pos.character >= *chars_since_newline;
                        let below_upper_bound = pos.character < *chars_since_newline + len;
                        if at_least_lower_bound && below_upper_bound {
                            return Ok(vec![i as u32]);
                        }
                    }
                    *chars_since_newline += len;
                }
                Expr::List(xs) => {
                    *chars_since_newline += 1; // account for '('
                    if let Ok(mut v) = xs.path_to_node_by_lsp_pos(pos, line, chars_since_newline) {
                        v.insert(0, i as u32);
                        return Ok(v);
                    }
                    *chars_since_newline += 1; // account for ')'
                }
            };

            for m in post_metadata {
                *line += m.to_string().encode_utf16().fold(0, |acc, n| {
                    if n == b'\n' as u16 {
                        *chars_since_newline = 0;
                        acc + 1
                    } else {
                        *chars_since_newline += 1;
                        acc
                    }
                });
            }
            if *line > pos.line {
                return Err(anyhow!("position is inside metadata (2)"));
            }
        }
        Err(anyhow!("no match in this path"))
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[cfg(test)]
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
    pub file_content: &'a str, // used for utf8 <-> utf16 conversions
}

impl<'a> From<CustomSpan<'a>> for Span {
    fn from(val: CustomSpan<'a>) -> Self {
        Span {
            start: val.start,
            end: val.end,
            file_name: "".into(),
            file_content: val.file_content.into(),
        }
    }
}

/// Parses config from text, combining both [`SExpr`] and [`SExprMetaData`] into [`ExtParseTree`].
/// The result can be loselessly combined back into the original form.
pub fn parse_into_ext_tree_and_root_span(
    src: &str,
) -> std::result::Result<(ExtParseTree, CustomSpan<'_>), ParseError> {
    let (exprs, exprs_ext) = sexpr::parse_(src, "", false)?;
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
    use std::vec;

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

    #[test]
    fn test_ext_parse_tree_includes() {
        assert_eq!(
            parse_into_ext_tree("(include abc.kbd)")
                .expect("parses")
                .includes()
                .unwrap(),
            vec![PathBuf::from_str("abc.kbd").unwrap()]
        );
        assert_eq!(
            parse_into_ext_tree("(qwer abc.kbd)")
                .expect("parses")
                .includes()
                .unwrap(),
            Vec::<PathBuf>::new()
        );
        assert_eq!(
            parse_into_ext_tree("(include abc.kbd)(include 123.kbd)")
                .expect("parses")
                .includes()
                .unwrap(),
            vec![
                PathBuf::from_str("abc.kbd").unwrap(),
                PathBuf::from_str("123.kbd").unwrap(),
            ]
        );
        assert_eq!(
            parse_into_ext_tree("(include \"my config.kbd\")(include \"included file 123.kbd\")")
                .expect("parses")
                .includes()
                .unwrap(),
            vec![
                PathBuf::from_str("my config.kbd").unwrap(),
                PathBuf::from_str("included file 123.kbd").unwrap(),
            ]
        );
    }

    #[test]
    fn test_ext_parse_tree_multiple_filenames() {
        let r = parse_into_ext_tree("(include abc.kbd 123.kbd)")
            .expect("parses")
            .includes();
        assert!(r.is_err());
    }

    #[test]
    fn test_path_to_node_by_lsp_position_oneline() {
        let test_table = [
            (0, 0, false, vec![]),       // "("
            (0, 1, false, vec![]),       // "("
            (0, 2, true, vec![0, 0, 0]), // "1"
            (0, 3, false, vec![]),       // " "
            (0, 4, true, vec![0, 0, 1]), // "2"
            (0, 5, false, vec![]),       // ")"
            (0, 6, false, vec![]),       // " "
            (0, 7, true, vec![0, 1]),    // "3"
            (0, 8, true, vec![0, 1]),    // "4"
            (0, 9, true, vec![0, 1]),    // "5"
            (0, 10, false, vec![]),      // " "
            (0, 11, false, vec![]),      // " "
            (0, 12, true, vec![0, 2]),   // "6"
            (0, 13, false, vec![]),      // ")"
            (0, 14, false, vec![]),      // eof
        ];

        for ref test @ (line, char, expect_ok, ref expected_arr) in test_table {
            dbg!(&test);
            let pos: lsp_types::Position = lsp_types::Position::new(line, char);
            let r = parse_into_ext_tree("((1 2) 345  6)")
                .expect("should parse")
                .path_to_node_by_lsp_position(pos);

            if expect_ok {
                let r = r.expect("finds path");
                assert_eq!(r, *expected_arr);
            } else {
                r.expect_err("should error, because it's out of node bounds but it returned Ok");
            }
        }
    }

    #[test]
    fn test_path_to_node_by_lsp_position_multiline() {
        let pos: lsp_types::Position = lsp_types::Position::new(4, 3);
        let r = parse_into_ext_tree("\n(1\n) \n ;; comment \n\t (2) ")
            .expect("parses")
            .path_to_node_by_lsp_position(pos)
            .expect("finds path");
        assert_eq!(r, vec![1, 0]);
    }
}
