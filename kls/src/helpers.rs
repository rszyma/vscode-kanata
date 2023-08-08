// use std::collections::{BTreeMap, HashSet};

use std::rc::Rc;

use lsp_types::{PublishDiagnosticsParams, TextDocumentItem};

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

pub(crate) fn empty_diagnostics_for_doc(doc: &TextDocumentItem) -> PublishDiagnosticsParams {
    PublishDiagnosticsParams::new(doc.uri, vec![], Some(doc.version))
}

pub fn utf16_length(str: impl AsRef<str>) -> usize {
    let utf16_encoded: Vec<u16> = str.as_ref().encode_utf16().collect();
    utf16_encoded.len()
}

pub fn slice_rc_str(rc_str: &Rc<str>, start: usize, end: usize) -> String {
    (&rc_str[start..end]).to_string()
}

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

// pub fn path_to_url() {
//     let file_url = if filepath.is_absolute() {
//         Url::from_str(format!("file://{}", filepath.to_string_lossy()).as_ref())
//             .map_err(|_| INVALID_PATH_ERROR.to_string())?
//     } else {
//         match root_folder {
//             Some(root) => {
//                 Url::join(&root, &filepath.to_string_lossy()).map_err(|e| e.to_string())?
//             }
//             None => match all_documents.first_key_value() {
//                 Some(entry) => entry.0.to_owned(),
//                 None => return Err("No kanata files are opened".to_string()),
//             },
//         }
//     };
// }
