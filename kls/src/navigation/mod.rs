use std::collections::HashMap;

use lsp_types::{Position, Range, Url};

use crate::{
    helpers::{lsp_range_from_span, DefinitionLocations, ReferenceKind, ReferenceLocations},
    log,
};

#[derive(Debug)]
pub struct GotoDefinitionLink {
    pub source_range: Range,
    pub target_range: Range,
    pub target_filename: String,
}

pub fn goto_definition_for_token_at_pos(
    pos: &Position,
    source_doc: &Url,
    definition_locations_by_doc: &HashMap<Url, DefinitionLocations>,
    reference_locations_by_doc: &HashMap<Url, ReferenceLocations>,
    search_all_docs: bool,
) -> Option<GotoDefinitionLink> {
    let source_doc_reference_locations = reference_locations_by_doc.get(source_doc)?;

    let definition_loc =
        source_doc_reference_locations.definition_for_reference_at_position(pos)?;
    log!("{:?}", &definition_loc);

    let mut map: HashMap<Url, DefinitionLocations> = HashMap::new(); // todo: inline in else clause?
    let definitions_per_file: std::collections::hash_map::Iter<Url, DefinitionLocations> =
        if search_all_docs {
            definition_locations_by_doc.iter()
        } else {
            let item: DefinitionLocations =
                definition_locations_by_doc.get(source_doc).unwrap().clone();
            map.insert(source_doc.clone(), item);
            map.iter()
        };
    for (_, defs_in_file) in definitions_per_file {
        use ReferenceKind::*;
        let location_map = match definition_loc.ref_kind {
            Alias => &defs_in_file.0.alias,
            Variable => &defs_in_file.0.variable,
            VirtualKey => &defs_in_file.0.virtual_key,
            Layer => &defs_in_file.0.layer,
            Template => &defs_in_file.0.template,
            Include => {
                // Short cirtuit since we target_range will be always zero.
                return {
                    Some(GotoDefinitionLink {
                        source_range: definition_loc.source_range,
                        target_range: Range::default(),
                        target_filename: definition_loc.ref_name,
                    })
                };
            }
        };

        let loc = location_map
            .get(&definition_loc.ref_name)
            .map(|span| GotoDefinitionLink {
                source_range: definition_loc.source_range,
                target_range: lsp_range_from_span(span),
                target_filename: span.file_name(),
            });
        if loc.is_none() {
            continue;
        }

        // We're not checking definition locations in other files, because
        // there should be only 1 definition for an identifier anyway.
        return loc;
    }
    None
}

// Returns None if the token at given position is not a definition.
pub fn references_for_definition_at_pos(
    source_pos: &Position,
    source_doc: &Url,
    definition_locations_by_doc: &HashMap<Url, DefinitionLocations>,
    reference_locations_by_doc: &HashMap<Url, ReferenceLocations>,
    match_all_refs: bool, // Need to set `true` for workspace mode and `false` otherwise.
) -> Option<Vec<GotoDefinitionLink>> {
    let source_doc_definition_locations = definition_locations_by_doc.get(source_doc)?;

    let location_info =
        match source_doc_definition_locations.search_references_for_token_at_position(source_pos) {
            Some(x) => x,
            None => return None,
        };
    log!("{:?}", &location_info);

    let mut reference_links: Vec<GotoDefinitionLink> = Vec::new();

    let mut map: HashMap<Url, ReferenceLocations> = HashMap::new();
    let refs_iter: std::collections::hash_map::Iter<Url, ReferenceLocations> = if match_all_refs {
        reference_locations_by_doc.iter()
    } else {
        let item: ReferenceLocations = reference_locations_by_doc.get(source_doc).unwrap().clone();
        map.insert(source_doc.clone(), item);
        map.iter()
    };
    for (_, reference_locations) in refs_iter {
        use ReferenceKind::*;
        let location_map = match location_info.ref_kind {
            Alias => &reference_locations.0.alias,
            Variable => &reference_locations.0.variable,
            VirtualKey => &reference_locations.0.virtual_key,
            Layer => &reference_locations.0.layer,
            Template => &reference_locations.0.template,
            Include => unreachable!("includes can't be backreferenced"),
        };
        let locations: Option<Vec<_>> = location_map.0.get(&location_info.ref_name).map(|spans| {
            spans
                .iter()
                .map(|span| GotoDefinitionLink {
                    source_range: location_info.source_range,
                    target_range: lsp_range_from_span(span),
                    target_filename: span.file_name(),
                })
                .collect()
        });
        if let Some(locations) = locations {
            reference_links.extend(locations)
        }
    }
    if reference_links.is_empty() {
        return None;
    }
    Some(reference_links)
}
