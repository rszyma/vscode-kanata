use kanata_parser::cfg::sexpr::Span;
use lsp_types::{SemanticTokenModifier, SemanticTokenType};

/// Global enable/disable of certain semantic token types.
pub const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::TYPE,
    SemanticTokenType::CLASS,
    SemanticTokenType::ENUM,
    SemanticTokenType::INTERFACE,
    SemanticTokenType::STRUCT,
    SemanticTokenType::TYPE_PARAMETER,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::EVENT,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::MACRO,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::MODIFIER,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::REGEXP,
    SemanticTokenType::OPERATOR,
];

/// Global enable/disable of certain semantic token modifiers.
pub const SEMANTIC_TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    // SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
    // SemanticTokenModifier::READONLY,
    // SemanticTokenModifier::STATIC,
    // SemanticTokenModifier::DEPRECATED, // potentially could be useful I think
    // SemanticTokenModifier::ABSTRACT,
    // SemanticTokenModifier::ASYNC,
    // SemanticTokenModifier::MODIFICATION,
    // SemanticTokenModifier::DOCUMENTATION,
    // SemanticTokenModifier::DEFAULT_LIBRARY,
];

pub fn index_of_token_type(t: SemanticTokenType) -> Option<u32> {
    for (i, type_) in SEMANTIC_TOKEN_TYPES.iter().enumerate() {
        if type_ == &t {
            return Some(i as u32);
        }
    }
    None
}

fn index_of_token_modifier(t: SemanticTokenModifier) -> Option<u32> {
    for (i, type_) in SEMANTIC_TOKEN_MODIFIERS.iter().enumerate() {
        if type_ == &t {
            return Some(i as u32);
        }
    }
    None
}

pub fn bitset_of_token_modifiers(mods: &[SemanticTokenModifier]) -> u32 {
    mods.iter()
        .filter_map(|mod_| index_of_token_modifier(mod_.clone()))
        .fold(0, |acc, i| acc | 1 << i)
}

// #[test]
// fn bitset_of_token_modifiers_works() {
//     assert_eq!(
//         bitset_of_token_modifiers(&[
//             SemanticTokenModifier::DECLARATION,
//             SemanticTokenModifier::READONLY
//         ]),
//         5
//     );
// }

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct SemanticTokenWithAbsoluteRange {
    pub span: Span,
    pub token_type: u32,
    pub token_modifiers_bitset: u32,
}

#[macro_export]
macro_rules! push_defs {
    ($results:expr, $defs:expr, $token_type:ident, $token_modifiers:expr) => {
        #[allow(unused)]
        use $crate::semantic_tokens::*;
        if let Some(token_type_index) = index_of_token_type(SemanticTokenType::$token_type) {
            for (_, span) in $defs.iter() {
                $results.push(SemanticTokenWithAbsoluteRange {
                    span: span.clone(),
                    token_type: token_type_index,
                    token_modifiers_bitset: bitset_of_token_modifiers($token_modifiers),
                });
            }
        }
    };
}

#[macro_export]
macro_rules! push_refs {
    ($results:expr, $refs:expr, $token_type:ident, $token_modifiers:expr) => {
        #[allow(unused)]
        use $crate::semantic_tokens::*;
        if let Some(token_type_index) = index_of_token_type(SemanticTokenType::$token_type) {
            for (_, spans) in $refs.0.iter() {
                for span in spans.iter() {
                    $results.push(SemanticTokenWithAbsoluteRange {
                        span: span.clone(),
                        token_type: token_type_index,
                        token_modifiers_bitset: bitset_of_token_modifiers($token_modifiers),
                    });
                }
            }
        }
    };
}
