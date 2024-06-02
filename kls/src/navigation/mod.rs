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

pub fn definition_location(
    pos: &Position,
    source_doc: &Url,
    definition_locations_by_doc: &HashMap<Url, DefinitionLocations>,
    reference_locations_by_doc: &HashMap<Url, ReferenceLocations>,
) -> Option<GotoDefinitionLink> {
    let source_doc_reference_locations = reference_locations_by_doc.get(source_doc)?;

    let location_info = match source_doc_reference_locations.search_definitions_at_position(pos) {
        Some(x) => x,
        None => return None,
    };
    log!("{:?}", &location_info);

    for (_, definition_locations) in definition_locations_by_doc.iter() {
        use ReferenceKind::*;
        let location_map = match location_info.ref_kind {
            Alias => &definition_locations.0.alias,
            Variable => &definition_locations.0.variable,
            VirtualKey => &definition_locations.0.virtual_key,
            ChordGroup => &definition_locations.0.chord_group,
            Layer => &definition_locations.0.layer,
            Include => {
                return {
                    Some(GotoDefinitionLink {
                        source_range: location_info.source_range,
                        target_range: Range::default(),
                        target_filename: location_info.ref_name,
                    })
                };
            }
        };

        let loc = location_map
            .get(&location_info.ref_name)
            .map(|span| GotoDefinitionLink {
                source_range: location_info.source_range,
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

pub fn references(
    pos: &Position,
    source_doc: &Url,
    definition_locations_by_doc: &HashMap<Url, DefinitionLocations>,
    reference_locations_by_doc: &HashMap<Url, ReferenceLocations>,
) -> Option<Vec<GotoDefinitionLink>> {
    let source_doc_definition_locations = definition_locations_by_doc.get(source_doc)?;

    let location_info = match source_doc_definition_locations.search_references_at_position(pos) {
        Some(x) => x,
        None => return None,
    };
    log!("{:?}", &location_info);

    let mut reference_links: Vec<GotoDefinitionLink> = Vec::new();

    for (_, reference_locations) in reference_locations_by_doc.iter() {
        use ReferenceKind::*;
        let location_map = match location_info.ref_kind {
            Alias => &reference_locations.0.alias,
            Variable => &reference_locations.0.variable,
            VirtualKey => &reference_locations.0.virtual_key,
            ChordGroup => &reference_locations.0.chord_group,
            Layer => &reference_locations.0.layer,
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
