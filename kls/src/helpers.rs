use std::{collections::BTreeMap, path::Path, rc::Rc};

use kanata_parser::cfg::{sexpr::Span, FileContentProvider, ParseError};
use lsp_types::{PublishDiagnosticsParams, TextDocumentItem, Url};

pub type HashSet<T> = rustc_hash::FxHashSet<T>;

pub type Documents = BTreeMap<Url, TextDocumentItem>;
pub type Diagnostics = BTreeMap<Url, PublishDiagnosticsParams>;

#[cfg(target_os = "unknown")]
#[macro_export]
macro_rules! log {
    ($string:expr) => {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from($string))
    };
    ($($tokens:tt)*) => {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from(format!($($tokens)*)))
    };
}

#[cfg(not(target_os = "unknown"))]
#[macro_export]
macro_rules! log {
    ($($tokens:tt)*) => {
        println!($($tokens)*)
    };
}

#[cfg(target_os = "unknown")]
#[allow(dead_code)]
pub fn now() -> zduny_wasm_timer::Instant {
    zduny_wasm_timer::Instant::now()
}

#[cfg(not(target_os = "unknown"))]
#[allow(dead_code)]
pub fn now() -> std::time::Instant {
    std::time::Instant::now()
}

pub(crate) fn empty_diagnostics_for_doc(
    (uri, doc): (&Url, &TextDocumentItem),
) -> (Url, PublishDiagnosticsParams) {
    let params = PublishDiagnosticsParams::new(uri.clone(), vec![], Some(doc.version));
    (uri.clone(), params)
}

pub fn utf16_length(str: impl AsRef<str>) -> usize {
    let utf16_encoded: Vec<u16> = str.as_ref().encode_utf16().collect();
    utf16_encoded.len()
}

pub fn slice_rc_str(rc_str: &Rc<str>, start: usize, end: usize) -> &str {
    &rc_str[start..end]
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
            span: e.span.unwrap_or_else(|| Span {
                file_name: main_cfg_file.into(),
                ..Default::default()
            }),
        }
    }
}

pub fn lsp_range_from_span(span: &Span) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position::new(
            span.start.line.try_into().unwrap(),
            utf16_length(slice_rc_str(
                &span.file_content,
                span.start.line_beginning,
                span.start.absolute,
            ))
            .try_into()
            .unwrap(),
        ),
        end: lsp_types::Position::new(
            span.end.line.try_into().unwrap(),
            utf16_length(slice_rc_str(
                &span.file_content,
                span.end.line_beginning,
                span.end.absolute,
            ))
            .try_into()
            .unwrap(),
        ),
    }
}

pub fn parse_wrapper(
    main_cfg_text: &str,
    main_cfg_path: &Path,
    file_content_provider: &mut FileContentProvider,
    def_local_keys_variant_to_apply: &str,
) -> Result<(), CustomParseError> {
    kanata_parser::cfg::parse_cfg_raw_string(
        main_cfg_text,
        &mut kanata_parser::cfg::ParsedState::default(),
        main_cfg_path,
        file_content_provider,
        def_local_keys_variant_to_apply,
    )
    .map(|_| {
        log!(
            "parsed file `{}` without errors",
            main_cfg_path.to_string_lossy(),
        );
        // Ignoring the non-error parser result for now.
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
