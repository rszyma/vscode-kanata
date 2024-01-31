use super::{parse_into_ext_tree_and_root_span, ExtParseTree};
use crate::{path_to_url, WorkspaceOptions};
use anyhow::{anyhow, Ok};
use lsp_types::{TextDocumentItem, Url};
use std::{collections::BTreeMap, iter, path::PathBuf, str::FromStr};

pub fn get_defsrc_layout(
    workspace_options: &WorkspaceOptions,
    documents: &BTreeMap<Url, TextDocumentItem>,
    tab_size: u32,
    file_uri: &Url,      // of current file
    tree: &ExtParseTree, // of current file
) -> anyhow::Result<Option<Vec<Vec<usize>>>> {
    match workspace_options {
        WorkspaceOptions::Single => {
            if tree.includes()?.is_empty() {
                tree.defsrc_layout(tab_size)
            } else {
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

            let main_tree: ExtParseTree = if main_config_file_url == *file_uri {
                // currently opened file is the main file
                tree.clone() // TODO: prevent clone
            } else {
                // currently opened file is non-main file, and probably an included file.
                let text = &documents
                    .get(file_uri)
                    .map(|doc| &doc.text)
                    .ok_or_else(|| {
                        anyhow!(
                            "included file is not present in the workspace: {}",
                            file_uri.to_string()
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
            let mut defsrc_layout = None;
            for file_url in includes.iter().chain(iter::once(&main_config_file_url)) {
                let text = &documents
                    .get(file_url)
                    .expect("document should be cached")
                    .text;

                let tree = parse_into_ext_tree_and_root_span(text)
                    .map(|x| x.0)
                    .map_err(|e| {
                        anyhow!(
                            "parse_into_ext_tree_and_root_span failed for file '{file_uri}': {}",
                            e.msg
                        )
                    })?;

                if let Some(layout) = tree
                    .defsrc_layout(tab_size)
                    .map_err(|e| anyhow!("tree.defsrc_layout for '{file_url}' failed: {e}"))?
                {
                    defsrc_layout = Some(layout);
                }
            }
            Ok(defsrc_layout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_no_includes() {
        let src = "(defsrc 1 2) (deflayer base 3 4)";
        let layout = get_defsrc_layout(
            &WorkspaceOptions::Single,
            &BTreeMap::new(),
            4,
            &Url::from_str("file:///main.kbd").unwrap(),
            &parse_into_ext_tree_and_root_span(src).unwrap().0,
        )
        .unwrap()
        .ok_or("should be some")
        .unwrap();

        assert_eq!(layout, vec![vec![3], vec![1]]);
    }

    #[test]
    fn single_with_includes() {
        let src = "(defsrc 1 2) (deflayer base 3 4) (include file.kbd)";
        let _ = get_defsrc_layout(
            &WorkspaceOptions::Single,
            &BTreeMap::new(),
            4,
            &Url::from_str("file:///main.kbd").unwrap(),
            &parse_into_ext_tree_and_root_span(src).unwrap().0,
        )
        .expect_err("should be error, because includes don't work in Single mode");
    }
}
