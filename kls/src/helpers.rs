// use std::collections::{BTreeMap, HashSet};

use std::rc::Rc;

use lsp_types::{PublishDiagnosticsParams, TextDocumentItem, Url};

pub type HashSet<T> = rustc_hash::FxHashSet<T>;
// type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;

#[macro_export]
macro_rules! log {
    ($string:expr) => {
        web_sys::console::log_1(&JsValue::from($string))
    };
    ($($tokens:tt)*) => {
        web_sys::console::log_1(&JsValue::from(format!($($tokens)*)))
    };
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

pub fn slice_rc_str(rc_str: &Rc<str>, start: usize, end: usize) -> String {
    (&rc_str[start..end]).to_string()
}
