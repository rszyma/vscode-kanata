extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use crate::{
    formatter::defsrc_layout::LineEndingSequence,
    helpers::{lsp_range_from_span, HashSet},
};
use anyhow::{anyhow, bail};
use formatter::Formatter;
use kanata_parser::cfg::{sexpr::Span, FileContentProvider, LspHintInactiveCode, ParseError};
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidDeleteFiles,
        DidOpenTextDocument, DidSaveTextDocument, Initialized, Notification,
    },
    request::{Formatting, Request},
    DeleteFilesParams, Diagnostic, DiagnosticSeverity, DiagnosticTag, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, FileChangeType, FileDelete, FileEvent, InitializeParams,
    PublishDiagnosticsParams, TextDocumentItem, TextEdit, Url, VersionedTextDocumentIdentifier,
};
use serde::Deserialize;
use serde_wasm_bindgen::{from_value, to_value};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    path::{self, Path, PathBuf},
    str::{FromStr, Split},
};
use wasm_bindgen::prelude::*;

mod helpers;
use helpers::{
    empty_diagnostics_for_doc, parse_wrapper, CustomParseError, Diagnostics, Documents,
    KlsParserOutput,
};

mod formatter;

struct Kanata {
    def_local_keys_variant_to_apply: String,
    env_vars: Vec<(String, String)>,
}

const EXTENSION_ERROR_PREFIX: &str = "Kanata Extension: ";
fn kanata_extension_error(err_msg: impl AsRef<str>) -> String {
    format!(r"{}{}", EXTENSION_ERROR_PREFIX, err_msg.as_ref(),)
}

const KANATA_PARSER_HELP: &str = r"For more info, see the configuration guide or ask in GitHub discussions.
    guide: https://github.com/jtroo/kanata/blob/main/docs/config.adoc
    ask: https://github.com/jtroo/kanata/discussions";

impl Kanata {
    fn new(
        def_local_keys_variant_to_apply: DefLocalKeysVariant,
        env_vars: Vec<(String, String)>,
    ) -> Self {
        #[cfg(target_os = "unknown")] // todo: make this compilable for non-wasm too
        {
            *kanata_parser::keys::OSCODE_MAPPING_VARIANT.lock() =
                match def_local_keys_variant_to_apply {
                    DefLocalKeysVariant::Win
                    | DefLocalKeysVariant::Wintercept
                    | DefLocalKeysVariant::WinIOv2 => kanata_parser::keys::Platform::Win,
                    DefLocalKeysVariant::Linux => kanata_parser::keys::Platform::Linux,
                    DefLocalKeysVariant::MacOS => kanata_parser::keys::Platform::Macos,
                };
        }
        Self {
            def_local_keys_variant_to_apply: def_local_keys_variant_to_apply.to_string(),
            env_vars,
        }
    }

    /// Parses with includes disabled.
    fn parse_single_file(
        &self,
        main_cfg_filename: &Path, // will be used only as filename in spans.
        main_cfg_text: &str,
        // Indicates whether the file is actually opened in VS Code workspace or, not.
        // regardles of what is WorkspaceOptions config option set to.
        is_opened_in_workspace: bool,
    ) -> KlsParserOutput {
        let mut get_file_content_fn_impl = |_: &Path| {
            if is_opened_in_workspace {
                Err(kanata_extension_error(["Includes currently can't be analyzed, because the support for it is disabled in the extension settings.",
                    "If you want to enable `includes` support, you need to:",
                    "\t1. Go to the settings in VS Code (File > Preferences > Settings)",
                    "\t2. Navigate to vscode-kanata settings: (Extensions > Kanata)",
                    "\t3. Change `Includes And Workspaces` to `workspace`"].join("\n")))
            } else {
                Err(kanata_extension_error(
                    "Includes can't be analyzed, because the current file is not opened in a workspace. Please, open the containing folder (File > Open Folder).",
                ))
            }
        };

        parse_wrapper(
            main_cfg_text,
            main_cfg_filename,
            &mut FileContentProvider::new(&mut get_file_content_fn_impl),
            &self.def_local_keys_variant_to_apply,
            &self.env_vars,
        )
    }

    fn parse_workspace(
        &self,
        root_folder: &Url,
        main_cfg_file: &Path,
        all_documents: &Documents,
    ) -> KlsParserOutput {
        log!(
            "kanata.parse_workspace for main_cfg_file={:?}",
            main_cfg_file
        );

        const INVALID_PATH_ERROR: &str = "The provided config file path is not valid";

        let mut loaded_files: HashSet<Url> = HashSet::default();

        let mut get_file_content_fn_impl = |filepath: &Path| {
            let file_url = path_to_url(filepath, root_folder).map_err(|_| INVALID_PATH_ERROR)?;

            log!("searching URL across opened documents: {}", file_url);
            let doc = all_documents.get(&file_url).ok_or_else(|| {
                kanata_extension_error("Can't open this file for analysis, because it doesn't exist, or is outside of opened workspace.")
            })?;

            if !loaded_files.insert(file_url) {
                return Err("The provided config file was already included before".to_string());
            }

            Ok(doc.text.clone())
        };

        let mut file_content_provider = FileContentProvider::new(&mut get_file_content_fn_impl);

        let text_or_not = &file_content_provider
            .get_file_content(main_cfg_file)
            .map_err(|e| {
                CustomParseError::from_parse_error(
                    ParseError::new_without_span(e),
                    main_cfg_file.to_string_lossy().to_string().as_str(),
                )
            });

        let text = match text_or_not {
            Ok(text) => text,
            Err(err) => {
                return KlsParserOutput {
                    errors: vec![err.clone()],
                    inactive_codes: vec![],
                }
            }
        };

        parse_wrapper(
            text,
            main_cfg_file,
            &mut file_content_provider,
            &self.def_local_keys_variant_to_apply,
            &self.env_vars,
        )
    }
}

fn path_to_url(path: &Path, root_folder: &Url) -> anyhow::Result<Url> {
    let file_url = if path.is_absolute() {
        Url::from_str(format!("file://{}", path.to_string_lossy()).as_ref())
            .map_err(|_| anyhow!("invalid path"))?
    } else {
        Url::join(root_folder, &path.to_string_lossy())?
    };
    Ok(file_url)
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(rename = "includesAndWorkspaces")]
    includes_and_workspaces: IncludesAndWorkspaces,
    #[serde(rename = "mainConfigFile")]
    main_config_file: String,
    #[serde(rename = "localKeysVariant")]
    def_local_keys_variant: DefLocalKeysVariant,
    format: ExtensionFormatterOptions,
    #[serde(rename = "envVariables")]
    env_variables: HashMap<String, String>,
    #[serde(rename = "dimInactiveConfigItems")]
    dim_inactive_config_items: bool,
}

#[derive(Debug, Deserialize, Clone, Copy)]
enum IncludesAndWorkspaces {
    #[serde(rename = "single")]
    Single,
    #[serde(rename = "workspace")]
    Workspace,
}

#[derive(Debug, Deserialize, Clone, Copy)]
enum DefLocalKeysVariant {
    #[serde(rename = "deflocalkeys-win")]
    Win,
    #[serde(rename = "deflocalkeys-wintercept")]
    Wintercept,
    #[serde(rename = "deflocalkeys-linux")]
    Linux,
    #[serde(rename = "deflocalkeys-macos")]
    MacOS,
    #[serde(rename = "deflocalkeys-winiov2")]
    WinIOv2,
}

impl Display for DefLocalKeysVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefLocalKeysVariant::Win => f.write_str("deflocalkeys-win"),
            DefLocalKeysVariant::Wintercept => f.write_str("deflocalkeys-wintercept"),
            DefLocalKeysVariant::Linux => f.write_str("deflocalkeys-linux"),
            DefLocalKeysVariant::MacOS => f.write_str("deflocalkeys-macos"),
            DefLocalKeysVariant::WinIOv2 => f.write_str("deflocalkeys-winiov2"),
        }
    }
}

#[derive(Debug, Clone)]
enum WorkspaceOptions {
    /// `root` is `None` in Single mode when file is not opened in a workspace.
    Single {
        root: Option<Url>,
    },
    Workspace {
        main_config_file: String,
        root: Url,
    },
}

impl WorkspaceOptions {
    fn from_config(config: &Config, root_folder: Option<Url>) -> Self {
        match config.includes_and_workspaces {
            IncludesAndWorkspaces::Single => WorkspaceOptions::Single { root: root_folder },
            IncludesAndWorkspaces::Workspace => WorkspaceOptions::Workspace {
                main_config_file: config.main_config_file.clone(),
                root: root_folder.expect("root folder should be set in workspace mode"),
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
// "interface FormatterSettings" in TS server.
pub struct ExtensionFormatterOptions {
    enable: bool,
    use_defsrc_layout_on_deflayers: bool,
}

#[wasm_bindgen]
pub struct KanataLanguageServer {
    documents: Documents,
    kanata: Kanata,
    workspace_options: WorkspaceOptions,
    send_diagnostics_callback: js_sys::Function,
    formatter: formatter::Formatter,
    dim_inactive_config_items: bool,
}

/// Public API exposed via WASM.
#[wasm_bindgen]
impl KanataLanguageServer {
    #[wasm_bindgen(constructor)]
    pub fn new(initialize_params: JsValue, send_diagnostics_callback: &js_sys::Function) -> Self {
        console_error_panic_hook::set_once();

        #[allow(deprecated)]
        let InitializeParams {
            mut root_uri,
            initialization_options,
            ..
        } = from_value(initialize_params).unwrap();

        let mut config: Config =
            serde_json::from_str(initialization_options.unwrap().to_string().as_str()).unwrap();

        log!("{:?}", &config);

        match &mut root_uri {
            Some(url) => {
                log!("workspace root: {}", url.as_ref().to_string());
                // Ensure the path ends with a slash
                if !url.path().ends_with('/') {
                    url.path_segments_mut()
                        .expect("Invalid path")
                        .pop_if_empty()
                        .push("");
                }
            }
            None => {
                log!("workspace root is not set, forcing `WorkspaceOptions::Single`.");
                config.includes_and_workspaces = IncludesAndWorkspaces::Single;
            }
        };

        let workspace_options = WorkspaceOptions::from_config(&config, root_uri);
        let env_vars: Vec<_> = config.env_variables.into_iter().collect();

        log!("env variables: {:?}", &env_vars);

        Self {
            documents: BTreeMap::new(),
            kanata: Kanata::new(config.def_local_keys_variant, env_vars),
            formatter: Formatter {
                options: config.format,
                remove_extra_empty_lines: false,
            },
            workspace_options,
            send_diagnostics_callback: send_diagnostics_callback.clone(),
            dim_inactive_config_items: config.dim_inactive_config_items,
        }

        // self_.reload_diagnostics_debouncer =
        //     Some(EventDebouncer::new(Duration::from_millis(200), move |_| {
        //         self_._reload_and_send_diagnostics_for_all_documents();
        //     }));
        // self_
    }

    /// Catch-all handler for notifications sent by the LSP client.
    ///
    /// This function receives a notification's `method` and `params` and dispatches to the
    /// appropriate handler function based on `method`.
    #[allow(unused_variables)]
    #[wasm_bindgen(js_class = KanataLanguageServer, js_name = onNotification)]
    pub fn on_notification(&mut self, method: &str, params: JsValue) {
        log!("notification: {}", method);

        match method {
            // Nothing to do when we receive the `Initialized` notification.
            Initialized::METHOD => (),
            DidOpenTextDocument::METHOD => {
                let DidOpenTextDocumentParams { text_document } = from_value(params).unwrap();

                log!("opening: {}", text_document.uri);
                if self.upsert_document(text_document).is_some() {
                    log!("reopened tracked doc");
                }

                let diagnostics = self.get_diagnostics();
                self.send_diagnostics(&diagnostics);
            }
            // We don't care when a document is closed -- we care about all Kanata files in a
            // workspace folder regardless of which ones remain open.
            DidCloseTextDocument::METHOD => (),
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = from_value(params).unwrap();

                // Ensure we receive full -- not incremental -- updates.
                assert_eq!(params.content_changes.len(), 1);
                let change = params.content_changes.into_iter().next().unwrap();
                assert!(change.range.is_none());

                let VersionedTextDocumentIdentifier { uri, version } = params.text_document;

                let updated_doc = TextDocumentItem::new(uri, "kanata".into(), version, change.text);

                let uri = updated_doc.uri.clone();
                if self.upsert_document(updated_doc).is_none() {
                    log!("updated untracked doc: {}", uri);
                }
            }

            // This is the type of event we'll receive when a Kanata file is deleted, either via the
            // VS Code UI (right-click delete) or otherwise (e.g., `rm file.kbd` in a terminal).
            // The event comes from the `deleteWatcher` file watcher in the extension client.
            DidChangeWatchedFiles::METHOD => {
                // todo: test this
                let DidChangeWatchedFilesParams { changes } = from_value(params).unwrap();
                let uris: Vec<_> = changes
                    .into_iter()
                    .map(|FileEvent { uri, typ }| {
                        assert_eq!(typ, FileChangeType::DELETED); // We only watch for `Deleted` events.
                        uri
                    })
                    .collect();

                self.on_did_change_watched_files(uris);
            }

            // This is the type of event we'll receive when *any* file or folder is deleted via the
            // VS Code UI (right-click delete). These events are triggered by the
            // `workspace.fileOperations.didDelete.filters[0].glob = '**'` capability we send from
            // the TS server -> client, which then sends us `didDelete` events for *all files and
            // folders within the current workspace*. This is how we are notified of directory
            // deletions that might contain Kanata files, since they won't get picked up by the
            // `deleteWatcher` created in the client for reasons elaborated below.
            //
            // We can ignore any Kanata file URIs received via this handler since they'll already be
            // covered by a corresponding `DidChangeWatchedFiles` event emitted by the
            // `deleteWatcher` file watcher in the extension client that watches for any
            // `**/*.kbd` files deleted in the current workspace.
            //
            // In this handler we only care about *non-Kanata* URIs, which we treat as potential
            // deletions of directories containing Kanata files since those won't get picked up by
            // the `deleteWatcher` due to [a limitation of VS Code's file watching
            // capabilities][0].
            //
            // [0]: https://github.com/microsoft/vscode/issues/60813
            DidDeleteFiles::METHOD => {
                // todo: test this
                let DeleteFilesParams { files } = from_value(params).unwrap();
                let mut deleted_uris: Vec<Url> = vec![];
                for FileDelete { uri } in files {
                    match Url::parse(&uri) {
                        Ok(uri) => deleted_uris.push(uri),
                        Err(e) => log!("failed to parse URI: {}", e),
                    }
                }

                for uri in deleted_uris {
                    log!("detected file deletion: {}", uri);
                    let removed_docs = self.remove_tracked_documents_in_dir(&uri);
                    if !removed_docs.is_empty() {
                        let diagnostics = self.get_diagnostics();
                        self.send_diagnostics(&diagnostics);
                    }
                }
            }

            DidSaveTextDocument::METHOD => {
                let _params: DidSaveTextDocumentParams = from_value(params).unwrap();
                let diagnostics = self.get_diagnostics();
                self.send_diagnostics(&diagnostics);
            }

            _ => log!("unsupported notification"),
        }
    }

    #[allow(unused_variables)]
    #[wasm_bindgen(js_class = KanataLanguageServer, js_name = onDocumentFormatting)]
    pub fn on_document_formatting(&mut self, params: JsValue) -> JsValue {
        type Params = <Formatting as Request>::Params;
        type Result = <Formatting as Request>::Result;
        let params = from_value::<Params>(params).expect("deserializes");
        to_value::<Result>(&self.on_document_formatting_impl(&params)).expect("no conversion error")
    }

    /// Returns None on error.
    fn on_document_formatting_impl(
        &mut self,
        params: &DocumentFormattingParams,
    ) -> Option<Vec<TextEdit>> {
        if !self.formatter.options.enable {
            log!("Formatting request received, but formatting is disabled in vscode-kanata settings.");
            return Some(vec![]);
        }

        let text = &self
            .documents
            .get(&params.text_document.uri)
            .expect("document should be cached")
            .text;

        let (mut tree, root_span) =
            match formatter::ext_tree::parse_into_ext_tree_and_root_span(text) {
                Ok(x) => x,
                Err(_) => {
                    log!("failed to parse current file into tree");
                    return None;
                }
            };

        let range = lsp_range_from_span(&root_span.into());

        let defsrc_layout = formatter::defsrc_layout::get_defsrc_layout(
            &self.workspace_options,
            &self.documents,
            params.options.tab_size,
            &params.text_document.uri,
            &tree,
        )
        .map_err(|e| {
            log!("format: get_defsrc_layout error: {}", e);
        })
        .unwrap_or(None);

        // FIXME: Generally, this shouldn't be hard-coded to LF, but I couldn't find
        // how to get line ending sequence for the current file and vscode adjusts
        // line endings automatically, so setting it to LF hopefully won't cause any issues.
        let line_endings = LineEndingSequence::LF;

        self.formatter.format(
            &mut tree,
            &params.options,
            defsrc_layout.as_deref(),
            line_endings,
        );

        Some(vec![TextEdit {
            range,
            new_text: tree.to_string(),
        }])
    }
}

/// Individual LSP notification handlers.
impl KanataLanguageServer {
    // This is (currently) only used to handle deletions of Kanata *files*. `DidChangeWatchedFiles`
    // events come from the `deleteWatcher` filesystem watcher in the extension client. Due to [a
    // limitation of VS Code's filesystem watcher][0], we don't receive deletion events for Kanata
    // files nested inside of a deleted directory. See corresponding comments on `DidDeleteFiles`
    // and `DidChangeWatchedFiles` in `KanataLanguageServer::on_notification`.
    //
    // [0]: https://github.com/microsoft/vscode/issues/60813
    fn on_did_change_watched_files(&mut self, uris: Vec<Url>) {
        for uri in uris {
            log!("deleting: {}", uri);

            // If this returns `None`, `uri` was already removed from the local set of tracked
            // documents. An easy way to encounter this is to right-click delete a Kanata file via
            // the VS Code UI, which races the `DidDeleteFiles` and `DidChangeWatchedFiles` events.
            if let Some(doc) = self.remove_document(&uri) {
                let diagnostics = self.empty_diagnostics_for_a_single_document(&doc);
                self.send_diagnostics(&diagnostics);
            } else {
                log!("cannot delete untracked doc");
            }
        }
    }
}

/// Helper methods.
impl KanataLanguageServer {
    fn upsert_document(&mut self, doc: TextDocumentItem) -> Option<TextDocumentItem> {
        self.documents.insert(doc.uri.clone(), doc)
    }

    fn remove_document(&mut self, uri: &Url) -> Option<TextDocumentItem> {
        self.documents.remove(uri)
    }
    /// Remove tracked docs inside `dir`. Returns documents that were removed.
    fn remove_tracked_documents_in_dir(&mut self, dir: &Url) -> Vec<TextDocumentItem> {
        let (in_removed_dir, _not_in_removed_dir): (Documents, Documents) =
            self.documents.clone().into_iter().partition(|(uri, _)| {
                // Zip pair of `Option<Split<char>>`s into `Option<(Split<char>, Split<char>)>`.
                let maybe_segments = dir.path_segments().zip(uri.path_segments());
                // Compare paths (`Split<char>`) by zipping them together and comparing pairwise.
                let compare_paths = |(l, r): (Split<_>, Split<_>)| l.zip(r).all(|(l, r)| l == r);
                // If all path segments match b/w dir & uri, uri is in dir and should be removed.
                maybe_segments.map_or(false, compare_paths)
            });
        in_removed_dir
            .iter()
            .map(|(url, doc)| {
                log!("tracked document got deleted: {}", url);
                self.remove_document(url);
                doc.to_owned()
            })
            .collect()
    }

    fn send_diagnostics(&self, diagnostics: &Diagnostics) {
        log!("sending diagnostics for {} files", diagnostics.len());
        let this = &JsValue::null();
        for params in diagnostics.values() {
            let params = &to_value(&params).unwrap();
            if let Err(e) = self.send_diagnostics_callback.call1(this, params) {
                log!("send_diagnostics params:\n{:?}\nJS error: {:?}", params, e);
            }
        }
    }

    fn document_from_span(&self, span: &Span) -> anyhow::Result<Option<TextDocumentItem>> {
        let url: Url = match &self.workspace_options {
            WorkspaceOptions::Workspace { root, .. }
            | WorkspaceOptions::Single { root: Some(root) } => {
                let filename = span.file_name();
                Url::join(root, &filename).map_err(|e| anyhow!(e.to_string()))?
            }
            WorkspaceOptions::Single { root: None } => match &self.documents.first_key_value() {
                Some(entry) => entry.0.to_owned(),
                None => bail!("no kanata files are opened"),
            },
        };
        if let Some(document) = self.documents.get(&url) {
            Ok(Some(document.clone()))
        } else {
            let tracked_docs_str = self
                .documents
                .keys()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            log!(
                "untracked doc: {}\nTracked: {:?}\nDiagnostic: {:?}",
                url.to_string(),
                tracked_docs_str,
                span
            );
            Err(anyhow!("untracked doc"))
        }
    }

    fn diagnostics_from_kanata_parse_error(
        &self,
        err: &CustomParseError,
    ) -> (Option<TextDocumentItem>, Vec<Diagnostic>) {
        let (message, severity) = (err.msg.clone(), DiagnosticSeverity::ERROR);

        let doc: Option<TextDocumentItem> =
            self.document_from_span(&err.span).unwrap_or_else(|e| {
                log!(
                    "Error in `document_from_kanata_diagnostic_context`: {:?}",
                    e
                );
                None
            });

        let is_extension_the_error_source = message.starts_with(EXTENSION_ERROR_PREFIX);

        let mut diagnostics = vec![];

        let range = lsp_range_from_span(&err.span);

        diagnostics.push(Diagnostic {
            range,
            severity: Some(severity),
            source: if is_extension_the_error_source {
                Some("vscode-kanata".to_string())
            } else {
                Some("kanata-parser".to_string())
            },
            message,
            ..Default::default()
        });

        if !is_extension_the_error_source {
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::INFORMATION),
                message: KANATA_PARSER_HELP.to_string(),
                ..Default::default()
            });
        }

        (doc, diagnostics)
    }

    fn diagnostics_from_inactive_code(
        &self,
        inactive: &LspHintInactiveCode,
    ) -> (Option<TextDocumentItem>, Vec<Diagnostic>) {
        let doc: Option<TextDocumentItem> =
            self.document_from_span(&inactive.span).unwrap_or_else(|e| {
                log!(
                    "`diagnostics_from_inactive_code`: document not found '{:?}'",
                    e
                );
                None
            });

        let mut diagnostics = vec![];

        let range = lsp_range_from_span(&inactive.span);

        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::HINT),
            source: Some("kanata-parser".to_string()),
            message: inactive.reason.clone(),
            tags: Some(vec![DiagnosticTag::UNNECESSARY]),
            ..Default::default()
        });

        (doc, diagnostics)
    }

    fn parse_workspace(&self, main_config_file: &str, root: &Url) -> KlsParserOutput {
        log!("parse_workspace for main_config_file={}", main_config_file);
        let pb = PathBuf::from(main_config_file);
        let main_cfg_file = pb.as_path();

        self.kanata
            .parse_workspace(root, main_cfg_file, &self.documents)
    }

    fn parse_a_single_file_in_workspace(&self, doc: &TextDocumentItem) -> KlsParserOutput {
        let url_path_str = doc.uri.path();
        let main_cfg_filename: PathBuf = path::PathBuf::from_str(url_path_str)
            .expect("shoudn't error because it comes from Url");
        let main_cfg_text: &str = &doc.text;
        let is_opened_in_workspace: bool = match &self.workspace_options {
            WorkspaceOptions::Workspace { .. } => true,
            WorkspaceOptions::Single { root } => root.is_some(),
        };
        self.kanata
            .parse_single_file(&main_cfg_filename, main_cfg_text, is_opened_in_workspace)
    }

    /// Returns empty diagnostics for all tracked docs.
    /// Note, that when publishing diagnostics, if a document is omitted,
    /// its diagnostics won't be cleared. If we want to clear diagnostics
    /// for that document, we need to set an empty array for that that doc.
    fn empty_diagnostics_for_all_documents(&self) -> Diagnostics {
        self.documents
            .iter()
            .map(empty_diagnostics_for_doc)
            .collect()
    }

    fn empty_diagnostics_for_a_single_document(&self, doc: &TextDocumentItem) -> Diagnostics {
        vec![empty_diagnostics_for_doc((&doc.uri, doc))]
            .into_iter()
            .collect()
    }

    /// Gets up-to-date diagnostics from kanata-parser.
    /// All previously set diagnostics will be cleared.
    fn get_diagnostics(&self) -> Diagnostics {
        let docs = self
            .documents
            .values()
            .map(|doc| doc.to_owned())
            .collect::<Vec<_>>();
        let docs: Vec<_> = docs.iter().collect();

        let (parse_errors, inactive_codes): (Vec<CustomParseError>, Vec<LspHintInactiveCode>) =
            match &self.workspace_options {
                WorkspaceOptions::Single { .. } => {
                    let mut errs = vec![];
                    let mut inactives = vec![];
                    for doc in docs {
                        let KlsParserOutput {
                            errors,
                            inactive_codes,
                        } = self.parse_a_single_file_in_workspace(doc);
                        errs.extend(errors);
                        inactives.extend(inactive_codes);
                    }
                    (errs, inactives)
                }
                WorkspaceOptions::Workspace {
                    main_config_file,
                    root,
                } => {
                    let KlsParserOutput {
                        errors,
                        inactive_codes,
                    } = self.parse_workspace(main_config_file, root);
                    (errors, inactive_codes)
                }
            };

        let new_error_diags = parse_errors
            .iter()
            .map(|e| self.diagnostics_from_kanata_parse_error(e))
            .fold(Diagnostics::new(), |mut acc, (doc_or_not, diag)| {
                match doc_or_not {
                    Some(doc) => {
                        log!("added diagnostic for document: {}", doc.uri.as_str());
                        let url: &Url = &doc.uri;

                        let mut diags = acc.get(url).map(|x| x.to_owned()).unwrap_or(
                            PublishDiagnosticsParams::new(
                                url.to_owned(),
                                vec![],
                                Some(doc.version),
                            ),
                        );

                        diags.diagnostics.extend(diag);
                        acc.insert(url.to_owned(), diags.to_owned());
                    }
                    None => {
                        // This shouldn't happen, as earlier we've made sure that spans
                        // without assigned file have instead assigned main file as fallback.
                        log!("skipped diagnostic not bound to any document: {:?}", diag);
                    }
                };
                acc
            });

        // TODO: merge with code above
        let new_inactive_codes_diags = inactive_codes
            .iter()
            .map(|span| self.diagnostics_from_inactive_code(span))
            .fold(Diagnostics::new(), |mut acc, (doc_or_not, diag)| {
                match doc_or_not {
                    Some(doc) => {
                        log!("added diagnostic for document: {}", doc.uri.as_str());
                        let url: &Url = &doc.uri;

                        let mut diags = acc.get(url).map(|x| x.to_owned()).unwrap_or(
                            PublishDiagnosticsParams::new(
                                url.to_owned(),
                                vec![],
                                Some(doc.version),
                            ),
                        );

                        diags.diagnostics.extend(diag);
                        acc.insert(url.to_owned(), diags.to_owned());
                    }
                    None => {
                        // This shouldn't happen, as earlier we've made sure that spans
                        // without assigned file have instead assigned main file as fallback.
                        log!("skipped diagnostic not bound to any document: {:?}", diag);
                    }
                };
                acc
            });

        let mut diagnostics = self.empty_diagnostics_for_all_documents();
        diagnostics.extend(new_error_diags);
        if self.dim_inactive_config_items {
            diagnostics.extend(new_inactive_codes_diags);
        }
        diagnostics
    }
}
