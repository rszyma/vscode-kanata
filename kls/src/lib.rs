use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::{FromStr, Split},
};

use anyhow::{anyhow, bail};

mod helpers;

use helpers::*;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidDeleteFiles,
        DidOpenTextDocument, DidSaveTextDocument, Initialized, Notification,
    },
    DeleteFilesParams, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    FileChangeType, FileDelete, FileEvent, InitializeParams, PublishDiagnosticsParams, Range,
    TextDocumentItem, Url, VersionedTextDocumentIdentifier,
};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use kanata_parser::cfg::{FileContentProvider, ParseError};

enum Either<A, B> {
    Left(A),
    Right(B),
}

struct Kanata {}

fn kanata_extension_error(err_msg: impl AsRef<str>) -> String {
    format!(r"Kanata Extension: {}", err_msg.as_ref(),)
}

impl Kanata {
    fn new() -> Self {
        Self {}
    }

    fn parse(
        &self,
        root_folder: &Option<Url>, // is None if file is opened without workspace.
        main_cfg_file: Either<&Path, &Url>,
        all_documents: &Documents,
        includes_setting: IncludesAndWorkspaces,
    ) -> Result<(), ParseError> {
        const INVALID_PATH_ERROR: &str = "The provided config file path is not valid";

        let mut loaded_files: HashSet<Url> = HashSet::default();

        let mut get_file_content_fn_impl = |filepath: &Path| {
            // Make the include paths relative to main config file instead of kanata executable.
            let file_url = if filepath.is_absolute() {
                Url::from_str(format!("file://{}", filepath.to_string_lossy()).as_ref())
                    .map_err(|_| INVALID_PATH_ERROR.to_string())?
            } else {
                match root_folder {
                    Some(root) => {
                        Url::join(&root, &filepath.to_string_lossy()).map_err(|e| e.to_string())?
                    }
                    None => match all_documents.first_key_value() {
                        Some(entry) => entry.0.to_owned(),
                        None => return Err("No kanata files are opened".to_string()),
                    },
                }
            };

            let doc = match includes_setting {
                IncludesAndWorkspaces::Single => return Err(kanata_extension_error(vec![
                    "Includes currently can't be analyzed, because the support for it is disabled in the extension settings.",
                    "If you want to enable `includes` support, you need to:",
                    "\t1. Go to the settings in VS Code (File > Preferences > Settings)",
                    "\t2. Navigate to vscode-kanata settings: (Extensions > Kanata)",
                    "\t3. Change `Includes And Workspaces` to `workspace`",
                ].join("\n"))),
                IncludesAndWorkspaces::Workspace => {
                    log!("searching URL ({}) across opened documents", file_url);
                    all_documents.get(&file_url).ok_or_else(|| {
                        if root_folder.is_some() {
                            kanata_extension_error("Can't open this file for analysis, because it doesn't exist, or it outside of opened workspace.")
                        } else {
                            kanata_extension_error("Included files can't be analyzed in non-workspace mode.")
                        }
                    })?
                }
            };

            if !loaded_files.insert(file_url) {
                return Err("The provided config file was already included before".to_string());
            }

            Ok(doc.text.clone())
        };

        let mut file_content_provider = FileContentProvider::new(&mut get_file_content_fn_impl);

        let cfg_file_name: PathBuf = match root_folder {
            Some(_root) => match main_cfg_file {
                // guaranted to be a single-segment path (just filename).
                Either::Left(path) => path.to_owned(),
                // this always is absolute path.
                Either::Right(url) => PathBuf::from(url.path()),
            },
            // this is going to return an absolute path.
            None => {
                let url = all_documents
                    .first_key_value()
                    .expect("should be validated before")
                    .0;
                PathBuf::from(url.path())
            }
        };

        let text = &file_content_provider
            .get_file_content(&cfg_file_name)
            .map_err(|e| ParseError::new_without_span(e))?;

        kanata_parser::cfg::parse_cfg_raw_string(
            text,
            &mut kanata_parser::cfg::ParsedState::default(),
            &cfg_file_name,
            &mut file_content_provider,
        )
        .map(|_| {
            // Ignoring the content of the parser result for now.
            ()
        })
    }
}

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(rename = "includesAndWorkspaces")]
    includes_and_workspaces: IncludesAndWorkspaces,
    #[serde(rename = "mainConfigFile")]
    main_config_file: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
enum IncludesAndWorkspaces {
    #[serde(rename = "single")]
    Single,
    #[serde(rename = "workspace")]
    Workspace,
}

pub(crate) type Documents = BTreeMap<Url, TextDocumentItem>;
pub(crate) type Diagnostics = BTreeMap<Url, PublishDiagnosticsParams>;

#[wasm_bindgen]
pub struct KanataLanguageServer {
    documents: Documents,
    kanata: Kanata,
    config: Config,
    root: Option<Url>,
    send_diagnostics_callback: js_sys::Function,
}

/// Public API exposed via WASM.
#[wasm_bindgen]
impl KanataLanguageServer {
    #[wasm_bindgen(constructor)]
    pub fn new(initialize_params: JsValue, send_diagnostics_callback: &js_sys::Function) -> Self {
        console_error_panic_hook::set_once();

        let InitializeParams {
            mut root_uri,
            initialization_options,
            ..
        } = from_value(initialize_params).unwrap();

        let config: Config =
            serde_json::from_str(initialization_options.unwrap().to_string().as_str()).unwrap();

        log!("{:?}", &config);

        if let Some(url) = &mut root_uri {
            // Ensure the path ends with a slash
            if !url.path().ends_with('/') {
                url.path_segments_mut()
                    .expect("Invalid path")
                    .pop_if_empty()
                    .push("");
            }
        }

        log!("root: {:?}", root_uri.as_ref().map(|url| url.to_string()));

        Self {
            documents: BTreeMap::new(),
            kanata: Kanata::new(),
            config,
            root: root_uri,
            send_diagnostics_callback: send_diagnostics_callback.clone(),
        }
    }

    /// Catch-all handler for notifications sent by the LSP client.
    ///
    /// This function receives a notification's `method` and `params` and dispatches to the
    /// appropriate handler function based on `method`.
    #[allow(unused_variables)]
    #[wasm_bindgen(js_class = KanataLanguageServer, js_name = onNotification)]
    pub fn on_notification(&mut self, method: &str, params: JsValue) {
        log!(method);

        match method {
            // Nothing to do when we receive the `Initialized` notification.
            Initialized::METHOD => (),
            DidOpenTextDocument::METHOD => {
                let DidOpenTextDocumentParams { text_document } = from_value(params).unwrap();

                log!("opening: {}", text_document.uri);
                if self.upsert_document(text_document).is_some() {
                    log!("reopened tracked doc");
                }

                let diagnostics = self.reload_diagnostics();
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
                let DidChangeWatchedFilesParams { changes } = from_value(params).unwrap();
                let uris: Vec<_> = changes
                    .into_iter()
                    .map(|FileEvent { uri, typ }| {
                        assert_eq!(typ, FileChangeType::DELETED); // We only watch for `Deleted` events.
                        uri
                    })
                    .collect();

                let diagnostics = self.on_did_change_watched_files(uris);
                self.send_diagnostics(&diagnostics);
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
                let DeleteFilesParams { files } = from_value(params).unwrap();
                let mut uris = vec![];
                for FileDelete { uri } in files {
                    match Url::parse(&uri) {
                        Ok(uri) => uris.push(uri),
                        Err(e) => log!("failed to parse URI: {}", e),
                    }
                }

                if let Some(diagnostics) = self.on_did_delete_files(uris) {
                    self.send_diagnostics(&diagnostics);
                }
            }

            DidSaveTextDocument::METHOD => {
                let _params: DidSaveTextDocumentParams = from_value(params).unwrap();

                let diagnostics = self.reload_diagnostics();
                self.send_diagnostics(&diagnostics);
            }

            _ => log!("unexpected notification"),
        }
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
    fn on_did_change_watched_files(&mut self, uris: Vec<Url>) -> Diagnostics {
        let mut diagnostics = Diagnostics::new();

        for uri in uris {
            log!("deleting: {}", uri);

            // If this returns `None`, `uri` was already removed from the local set of tracked
            // documents. An easy way to encounter this is to right-click delete a Kanata file via
            // the VS Code UI, which races the `DidDeleteFiles` and `DidChangeWatchedFiles` events.
            if let Some(removed) = self.remove_document(&uri) {
                let (_, empty_diagnostics) = empty_diagnostics_for_doc((&uri, &removed));
                if diagnostics.insert(uri, empty_diagnostics).is_some() {
                    log!("duplicate URIs in event payload");
                }
            } else {
                log!("cannot delete untracked doc");
            }
        }

        diagnostics.append(&mut self.reload_diagnostics());
        diagnostics
    }

    // Returns `None` if no Kanata files were deleted.
    fn on_did_delete_files(&mut self, uris: Vec<Url>) -> Option<Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        for uri in uris {
            // If `removed` is empty, `uri` wasn't a directory containing tracked Kanata files or
            // `uri` itself was a Kanata file that was already removed via `DidChangeWatchedFiles`.
            let removed = self.remove_documents_in_dir(&uri);
            if !removed.is_empty() {
                log!("deleting: {}", uri);

                for (uri, params) in removed {
                    log!("deleted: {}", uri);

                    // NOTE(gj): fairly sure this will never be true.
                    if diagnostics.insert(uri, params).is_some() {
                        log!("multiple deletions of same doc");
                    }
                }
            }
        }

        if diagnostics.is_empty() {
            None
        } else {
            diagnostics.append(&mut self.reload_diagnostics());
            Some(diagnostics)
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
    /// Remove tracked docs inside `dir`.
    fn remove_documents_in_dir(&mut self, dir: &Url) -> Diagnostics {
        let (in_removed_dir, not_in_removed_dir): (Documents, Documents) =
            self.documents.clone().into_iter().partition(|(uri, _)| {
                // Zip pair of `Option<Split<char>>`s into `Option<(Split<char>, Split<char>)>`.
                let maybe_segments = dir.path_segments().zip(uri.path_segments());
                // Compare paths (`Split<char>`) by zipping them together and comparing pairwise.
                let compare_paths = |(l, r): (Split<_>, Split<_>)| l.zip(r).all(|(l, r)| l == r);
                // If all path segments match b/w dir & uri, uri is in dir and should be removed.
                maybe_segments.map_or(false, compare_paths)
            });
        // Replace tracked docs w/ docs that aren't in the removed dir.
        self.documents = not_in_removed_dir;
        in_removed_dir
            .iter()
            .map(empty_diagnostics_for_doc)
            .collect()
    }

    fn send_diagnostics(&self, diagnostics: &Diagnostics) {
        let this = &JsValue::null();
        for params in diagnostics.values() {
            let params = &to_value(&params).unwrap();
            if let Err(e) = self.send_diagnostics_callback.call1(this, params) {
                log!("send_diagnostics params:\n{:?}\nJS error: {:?}", params, e);
            }
        }
    }

    fn empty_diagnostics_for_all_documents(&self) -> Diagnostics {
        self.documents
            .iter()
            .map(empty_diagnostics_for_doc)
            .collect()
    }

    fn document_from_kanata_diagnostic_context(
        &self,
        diagnostic: &ParseError,
    ) -> anyhow::Result<Option<TextDocumentItem>> {
        let url: Url = match &self.root {
            Some(root) => {
                let filename = match diagnostic.span.clone().map(|x| x.file_name()) {
                    Some(f) => f,
                    None => return Ok(None),
                };
                Url::join(&root, &filename).map_err(|e| anyhow!(e.to_string()))?
            }
            None => match &self.documents.first_key_value() {
                Some(entry) => entry.0.to_owned(),
                None => bail!("no kanata files are opened"),
            },
        };
        if let Some(document) = self.documents.get(&url) {
            Ok(Some(document.clone()))
        } else {
            let tracked_docs = self
                .documents
                .keys()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            log!(
                "untracked doc: {:?}\nTracked: {:?}\nDiagnostic: {:?}",
                url,
                tracked_docs,
                diagnostic
            );
            Err(anyhow!("untracked doc"))
        }
    }

    // Returns Range with UTF-16 column positions.
    fn lsp_range_from_kanata_diagnostic_context(&self, diagnostic: &ParseError) -> Range {
        let span = diagnostic.span.clone().unwrap_or_default();
        Range {
            start: lsp_types::Position::new(
                span.start.line.try_into().unwrap(),
                helpers::utf16_length(helpers::slice_rc_str(
                    &span.file_content,
                    span.start.line_beginning,
                    span.start.absolute,
                ))
                .try_into()
                .unwrap(),
            ),
            end: lsp_types::Position::new(
                span.end.line.try_into().unwrap(),
                helpers::utf16_length(helpers::slice_rc_str(
                    &span.file_content,
                    span.end.line_beginning,
                    span.end.absolute,
                ))
                .try_into()
                .unwrap(),
            ),
        }
    }

    fn diagnostics_from_kanata_parse_error(
        &self,
        diagnostic: &ParseError,
    ) -> Vec<(TextDocumentItem, Diagnostic)> {
        let (message, severity) = (&diagnostic.msg, DiagnosticSeverity::ERROR);

        // If the diagnostic applies to a single doc, use it; otherwise, default to emitting a
        // duplicate diagnostic for all docs.

        let doc = match self.document_from_kanata_diagnostic_context(&diagnostic) {
            Ok(x) => match x {
                Some(doc) => doc,
                None => {
                    log!("Diagnostic: {:?}", diagnostic);
                    return vec![];
                }
            },
            Err(e) => {
                log!("Error: {:?}", e);
                return vec![];
            }
        };

        vec![doc]
            .into_iter()
            .map(|doc| {
                let diagnostic = Diagnostic {
                    range: self.lsp_range_from_kanata_diagnostic_context(&diagnostic),
                    severity: Some(severity),
                    source: Some("vscode-kanata".to_owned()),
                    message: message.clone(),
                    ..Default::default()
                };
                (doc, diagnostic)
            })
            .collect()
    }

    fn parse_documents(&self) -> Vec<ParseError> {
        match self.config.includes_and_workspaces {
            IncludesAndWorkspaces::Single => self.documents.iter().fold(
                vec![],
                |mut acc: Vec<ParseError>, doc: (&Url, &TextDocumentItem)| {
                    let result = self.kanata.parse(
                        &self.root,
                        Either::Right(doc.0),
                        &self.documents,
                        self.config.includes_and_workspaces,
                    );
                    if let Err(e) = result {
                        acc.push(e)
                    };
                    acc
                },
            ),
            IncludesAndWorkspaces::Workspace => {
                let result = self.kanata.parse(
                    &self.root,
                    Either::Left(&PathBuf::from(self.config.main_config_file.clone())),
                    &self.documents,
                    self.config.includes_and_workspaces,
                );
                match result {
                    Ok(_) => vec![],
                    Err(e) => vec![e],
                }
            }
        }
    }

    fn get_diagnostics(&self) -> Diagnostics {
        self.parse_documents()
            .into_iter()
            .flat_map(|diagnostic| self.diagnostics_from_kanata_parse_error(&diagnostic))
            .fold(Diagnostics::new(), |mut acc, (doc, diagnostic)| {
                let params = acc.entry(doc.uri.clone()).or_insert_with(|| {
                    PublishDiagnosticsParams::new(doc.uri, vec![], Some(doc.version))
                });
                params.diagnostics.push(diagnostic);
                acc
            })
    }

    fn reload_diagnostics(&self) -> Diagnostics {
        let mut diagnostics = self.empty_diagnostics_for_all_documents();
        diagnostics.extend(self.get_diagnostics());
        diagnostics
    }
}
