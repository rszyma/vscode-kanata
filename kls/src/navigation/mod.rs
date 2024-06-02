use lsp_types::{Position, Range};

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

pub fn goto_definition(
    pos: &Position,
    identifier_locations: &DefinitionLocations,
    reference_locations: &ReferenceLocations,
) -> Option<GotoDefinitionLink> {
    let location_info = match reference_locations.search_definitions_at_position(pos) {
        Some(x) => x,
        None => return None,
    };
    log!("{:?}", &location_info);

    use ReferenceKind::*;
    let location_map = match location_info.ref_kind {
        Alias => &identifier_locations.0.alias,
        Variable => &identifier_locations.0.variable,
        VirtualKey => &identifier_locations.0.virtual_key,
        ChordGroup => &identifier_locations.0.chord_group,
        Layer => &identifier_locations.0.layer,
        Include => {
            return {
                // (ref_name, Range::default())
                // ref_name here is included file name
                Some(GotoDefinitionLink {
                    source_range: location_info.source_range,
                    target_range: Range::default(),
                    target_filename: location_info.ref_name,
                })
            };
        }
    };
    location_map
        .get(&location_info.ref_name)
        .map(|span| GotoDefinitionLink {
            source_range: location_info.source_range,
            target_range: lsp_range_from_span(span),
            target_filename: span.file_name(),
        })
}

pub fn references(
    pos: &Position,
    identifier_locations: &DefinitionLocations,
    reference_locations: &ReferenceLocations,
) -> Option<Vec<GotoDefinitionLink>> {
    let location_info = match identifier_locations.search_references_at_position(pos) {
        Some(x) => x,
        None => return None,
    };
    log!("{:?}", &location_info);

    use ReferenceKind::*;
    let location_map = match location_info.ref_kind {
        Alias => &reference_locations.0.alias,
        Variable => &reference_locations.0.variable,
        VirtualKey => &reference_locations.0.virtual_key,
        ChordGroup => &reference_locations.0.chord_group,
        Layer => &reference_locations.0.layer,
        Include => unreachable!("includes can't be backreferenced"),
    };

    location_map.0.get(&location_info.ref_name).map(|spans| {
        spans
            .iter()
            .map(|span| GotoDefinitionLink {
                source_range: location_info.source_range,
                target_range: lsp_range_from_span(span),
                target_filename: span.file_name(),
            })
            .collect()
    })
}
