use super::{parse_into_ext_tree_and_root_span, ExtParseTree};
use crate::{path_to_url, WorkspaceOptions};
use anyhow::{anyhow, Ok};
use lsp_types::{TextDocumentItem, Url};
use std::{collections::BTreeMap, iter, path::PathBuf, str::FromStr};

pub fn get_defsrc_keys(
    workspace_options: &WorkspaceOptions,
    documents: &BTreeMap<Url, TextDocumentItem>,
    file_uri: &Url,      // of current file
    tree: &ExtParseTree, // of current file
) -> anyhow::Result<Option<Vec<String>>> {
    match workspace_options {
        WorkspaceOptions::Single { .. } => {
            if tree.includes()?.is_empty() {
                tree.defsrc_keys()
            } else {
                // This is an error, because we don't know if those included files
                // and current file collectively don't contain >=2 `defsrc` blocks.
                // And if that's the case, we don't want to format `deflayers`.
                Err(anyhow!("includes are not supported in Single mode"))
            }
        }
        WorkspaceOptions::Workspace {
            main_config_file,
            root,
        } => {
            let main_config_file_path = PathBuf::from_str(main_config_file)
                .map_err(|e| anyhow!("main_config_file is an invalid path: {}", e))?;
            let main_config_file_url = path_to_url(&main_config_file_path, root)
                .map_err(|e| anyhow!("failed to convert main_config_file_path to url: {}", e))?;

            // Check if currently opened file is the main file.
            let main_tree: ExtParseTree = if main_config_file_url == *file_uri {
                tree.clone() // TODO: prevent clone
            } else {
                // Currently opened file is non-main file, it's probably an included file.
                let text = &documents
                    .get(&main_config_file_url)
                    .map(|doc| &doc.text)
                    .ok_or_else(|| {
                        anyhow!(
                            "included file is not present in the workspace: {}",
                            file_uri
                        )
                    })?;

                parse_into_ext_tree_and_root_span(text)
                    .map(|x| x.0)
                    .map_err(|e| anyhow!("parse_into_ext_tree_and_root_span failed: {}", e.msg))?
            };

            let includes = main_tree
                .includes()
                .map_err(|e| anyhow!("workspace [main = {main_config_file_url}]: {e}"))?
                .iter()
                .map(|path| path_to_url(path, root))
                .collect::<anyhow::Result<Vec<_>>>()
                .map_err(|e| anyhow!("path_to_url: {e}"))?;

            // make sure that all includes collectively contain only 1 defsrc
            let mut defsrc_keys = None;
            for file_url in includes.iter().chain(iter::once(&main_config_file_url)) {
                let doc = documents
                    .get(file_url)
                    .ok_or_else(|| anyhow!("document '{file_url}' is not loaded"))?;

                let tree = parse_into_ext_tree_and_root_span(&doc.text)
                    .map(|x| x.0)
                    .map_err(|e| {
                        anyhow!(
                            "parse_into_ext_tree_and_root_span failed for file '{file_uri}': {}",
                            e.msg
                        )
                    })?;

                if let Some(layout) = tree
                    .defsrc_keys()
                    .map_err(|e| anyhow!("tree.defsrc_keys for '{file_url}' failed: {e}"))?
                {
                    defsrc_keys = Some(layout);
                }
            }
            Ok(defsrc_keys)
        }
    }
}
