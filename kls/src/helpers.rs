// use std::collections::{BTreeMap, HashSet};

use std::{collections::BTreeMap, path::Path, rc::Rc};

use wasm_bindgen::JsValue;

use kanata_parser::cfg::{sexpr::Span, FileContentProvider, ParseError};
use lsp_types::{PublishDiagnosticsParams, TextDocumentItem, Url};

pub type HashSet<T> = rustc_hash::FxHashSet<T>;
// type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;

pub type Documents = BTreeMap<Url, TextDocumentItem>;
pub type Diagnostics = BTreeMap<Url, PublishDiagnosticsParams>;

#[macro_export]
macro_rules! log {
    ($string:expr) => {
        web_sys::console::log_1(&JsValue::from($string))
    };
    ($($tokens:tt)*) => {
        web_sys::console::log_1(&JsValue::from(format!($($tokens)*)))
    };
}

pub fn utf16_length(str: impl AsRef<str>) -> usize {
    let utf16_encoded: Vec<u16> = str.as_ref().encode_utf16().collect();
    utf16_encoded.len()
}

pub fn slice_rc_str(rc_str: &Rc<str>, start: usize, end: usize) -> String {
    (&rc_str[start..end]).to_string()
}

#[derive(Debug, Clone)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

#[derive(Debug, Clone)]
/// Compared to the span ParseError returned by kanata-parser
/// crate, this one has a non-optional span.
pub struct CustomParseError {
    pub msg: String,
    pub span: Span,
}

impl CustomParseError {
    pub fn from_parse_error(e: ParseError, main_cfg_file: &str) -> Self {
        Self {
            msg: e.msg,
            span: e.span.unwrap_or_else(|| {
                let mut span = Span::default();
                span.file_name = main_cfg_file.into();
                span
            }),
        }
    }
}

pub fn parse_wrapper(
    main_cfg_text: &str,
    main_cfg_path: &Path,
    file_content_provider: &mut FileContentProvider,
) -> Result<(), CustomParseError> {
    kanata_parser::cfg::parse_cfg_raw_string(
        main_cfg_text,
        &mut kanata_parser::cfg::ParsedState::default(),
        main_cfg_path.into(),
        file_content_provider,
    )
    .map(|_| {
        log!(
            "parsed file `{}` without errors",
            main_cfg_path.to_string_lossy(),
        );
        // Ignoring the non-error parser result for now.
        ()
    })
    .map_err(|e: ParseError| {
        CustomParseError::from_parse_error(e, main_cfg_path.to_string_lossy().to_string().as_str())
    })
    .map_err(|e| {
        log!(
            "parsing file `{}` resulted in error: `{}`",
            e.span.clone().file_name(),
            e.msg,
        );
        e
    })
}
