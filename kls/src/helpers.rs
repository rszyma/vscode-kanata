use std::{
    collections::BTreeMap,
    iter::{repeat, zip},
    path::Path,
    rc::Rc,
    str::FromStr,
};

use anyhow::anyhow;
use itertools::chain;
use kanata_parser::cfg::{sexpr::Span, FileContentProvider, ParseError};
use kanata_parser::lsp_hints::InactiveCode;
use lsp_types::{PublishDiagnosticsParams, Range, TextDocumentItem, Url};

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

#[derive(Clone, Copy, Debug)]
pub enum ReferenceKind {
    Alias,
    Variable,
    VirtualKey,
    Layer,
    Template,
    Include,
}

#[derive(Debug)]
pub struct LocationInfo {
    pub ref_kind: ReferenceKind,
    pub ref_name: String,
    pub source_range: Range,
}

#[derive(Debug, Default, Clone)]
pub struct DefinitionLocations(pub kanata_parser::lsp_hints::DefinitionLocations);

impl DefinitionLocations {
    pub fn search_references_for_token_at_position(
        &self,
        pos: &lsp_types::Position,
    ) -> Option<LocationInfo> {
        log!("looking for references @ {:?}", pos);
        for ((name, span), ref_kind) in chain!(
            zip(&self.0.alias, repeat(ReferenceKind::Alias)),
            zip(&self.0.variable, repeat(ReferenceKind::Variable)),
            zip(&self.0.virtual_key, repeat(ReferenceKind::VirtualKey)),
            zip(&self.0.layer, repeat(ReferenceKind::Layer)),
            zip(&self.0.template, repeat(ReferenceKind::Template)),
        ) {
            let range = lsp_range_from_span(span);
            if pos.line >= range.start.line
                && pos.line <= range.end.line
                && pos.character >= range.start.character
                && pos.character <= range.end.character
            {
                return Some(LocationInfo {
                    ref_kind,
                    ref_name: name.to_owned(),
                    source_range: range,
                });
            }
        }
        log!("search_references_at_position: not found any references");
        None
    }
}

#[derive(Debug, Default, Clone)]
pub struct ReferenceLocations(pub kanata_parser::lsp_hints::ReferenceLocations);

impl ReferenceLocations {
    pub fn definition_for_reference_at_position(
        &self,
        pos: &lsp_types::Position,
    ) -> Option<LocationInfo> {
        log!("looking for definition of token @ {:?}", pos);
        for ((name, spans), ref_kind) in chain!(
            zip(&self.0.alias.0, repeat(ReferenceKind::Alias)),
            zip(&self.0.variable.0, repeat(ReferenceKind::Variable)),
            zip(&self.0.virtual_key.0, repeat(ReferenceKind::VirtualKey)),
            zip(&self.0.layer.0, repeat(ReferenceKind::Layer)),
            zip(&self.0.template.0, repeat(ReferenceKind::Template)),
            zip(&self.0.include.0, repeat(ReferenceKind::Include)),
        ) {
            for span in spans {
                let range = lsp_range_from_span(span);
                if pos.line >= range.start.line
                    && pos.line <= range.end.line
                    && pos.character >= range.start.character
                    && pos.character <= range.end.character
                {
                    return Some(LocationInfo {
                        ref_kind,
                        ref_name: name.to_owned(),
                        source_range: range,
                    });
                }
            }
        }
        log!("search_definitions_at_position: not found any definitions");
        None
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

#[allow(clippy::large_enum_variant)] // not created that often
pub enum KlsParserOutput {
    Ok {
        inactive_codes: Vec<InactiveCode>,
        definition_locations: DefinitionLocations,
        reference_locations: ReferenceLocations,
    },
    Err {
        errors: Vec<CustomParseError>,
    },
}

pub fn parse_wrapper(
    main_cfg_text: &str,
    main_cfg_path: &Path,
    file_content_provider: &mut FileContentProvider,
    def_local_keys_variant_to_apply: &str,
    env_vars: &Vec<(String, String)>,
) -> KlsParserOutput {
    let parsed_state = &mut kanata_parser::cfg::ParserState::default();
    let result: anyhow::Result<KlsParserOutput> = kanata_parser::cfg::parse_cfg_raw_string(
        main_cfg_text,
        parsed_state,
        main_cfg_path,
        file_content_provider,
        def_local_keys_variant_to_apply,
        Ok(env_vars.to_owned()),
    )
    .map(|_| {
        log!(
            "parsed file `{}` without errors",
            main_cfg_path.to_string_lossy(),
        );
        KlsParserOutput::Ok {
            inactive_codes: parsed_state.lsp_hints.borrow().inactive_code.clone(),
            definition_locations: DefinitionLocations(
                parsed_state.lsp_hints.borrow().definition_locations.clone(),
            ),
            reference_locations: ReferenceLocations(
                parsed_state.lsp_hints.borrow().reference_locations.clone(),
            ),
        }
    })
    .or_else(|e: ParseError| {
        let e = CustomParseError::from_parse_error(
            e,
            main_cfg_path.to_string_lossy().to_string().as_str(),
        );
        log!(
            "parsing file `{}` resulted in error: `{}`",
            e.span.clone().file_name(),
            e.msg,
        );
        Ok(KlsParserOutput::Err { errors: vec![e] })
    });
    result.expect("no err")
}

pub fn path_to_url(path: &Path, root_folder: &Url) -> anyhow::Result<Url> {
    let file_url = if path.is_absolute() {
        Url::from_str(format!("file://{}", path.to_string_lossy()).as_ref())
            .map_err(|_| anyhow!("invalid path"))?
    } else {
        Url::join(root_folder, &path.to_string_lossy())?
    };
    Ok(file_url)
}

#[macro_export]
macro_rules! url_map_definitions {
    ($def_kind:ident, $root:expr, $definitions:expr, $definition_locations:expr) => {
        for (k, v) in $definition_locations.$def_kind.iter() {
            let url = match path_to_url(Path::new(v.file_name.as_ref()), $root) {
                Ok(url) => url,
                Err(e) => {
                    log!("path_to_url failed: {}", e);
                    continue;
                }
            };
            match $definitions.get_mut(&url) {
                Some(val) => {
                    val.0.$def_kind.insert(k.to_owned(), v.to_owned());
                }
                None => {
                    let mut def = kanata_parser::lsp_hints::DefinitionLocations::default();
                    def.$def_kind.insert(k.to_owned(), v.to_owned());
                    $definitions.insert(url, DefinitionLocations(def));
                }
            };
        }
    };
}

#[macro_export]
macro_rules! url_map_references {
    ($ref_kind:ident, $root:expr, $references:expr, $reference_locations:expr) => {
        for (k, spans) in $reference_locations.$ref_kind.0.iter() {
            for span in spans.iter() {
                let url = match path_to_url(Path::new(span.file_name.as_ref()), $root) {
                    Ok(url) => url,
                    Err(e) => {
                        log!("path_to_url failed: {}", e);
                        continue;
                    }
                };
                match $references.get_mut(&url) {
                    Some(refloc) => match refloc.0.$ref_kind.0.get_mut(k) {
                        Some(vec) => {
                            vec.push(span.to_owned());
                        }
                        None => {
                            refloc
                                .0
                                .$ref_kind
                                .0
                                .insert(k.to_owned(), vec![span.to_owned()]);
                        }
                    },
                    None => {
                        let mut refloc = kanata_parser::lsp_hints::ReferenceLocations::default();
                        refloc
                            .$ref_kind
                            .0
                            .insert(k.to_owned(), vec![span.to_owned()]);
                        $references.insert(url, ReferenceLocations(refloc));
                    }
                };
            }
        }
    };
}
