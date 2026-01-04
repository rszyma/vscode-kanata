use std::{collections::HashMap, iter::repeat};

use itertools::Itertools;
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
    pub kind: ReferenceKind,
    pub name: String,
}

/// Checks if there exists a symbol at given location which has a definition.
/// If yes, returns a link to the definition along with metadata.
pub fn goto_definition_for_token_at_pos(
    pos: &Position,
    source_doc: &Url,
    definition_locations_by_doc: &HashMap<Url, DefinitionLocations>,
    reference_locations_by_doc: &HashMap<Url, ReferenceLocations>,
    search_all_docs: bool, // Need to be set `true` for workspace mode and `false` otherwise.
) -> Option<GotoDefinitionLink> {
    let source_doc_reference_locations = reference_locations_by_doc.get(source_doc)?;

    let definition_loc = source_doc_reference_locations.get_reference_at_position(pos)?;
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
                return {
                    Some(GotoDefinitionLink {
                        source_range: definition_loc.range,
                        target_range: Range::default(),
                        target_filename: definition_loc.ref_name.clone(),
                        kind: ReferenceKind::Include,
                        name: definition_loc.ref_name,
                    })
                };
            }
        };

        let loc = location_map
            .get(&definition_loc.ref_name)
            .map(|span| GotoDefinitionLink {
                source_range: definition_loc.range,
                target_range: lsp_range_from_span(span),
                target_filename: span.file_name(),
                kind: definition_loc.ref_kind,
                name: definition_loc.ref_name.clone(),
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
    let found_definition = definition_locations_by_doc
        .get(source_doc)?
        .get_definition_at_position(source_pos)?;

    log!(
        "found definition {:?} at position {:?} in file {}",
        &found_definition,
        &source_pos,
        source_doc
    );

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
        let location_map = match found_definition.ref_kind {
            Alias => &reference_locations.0.alias,
            Variable => &reference_locations.0.variable,
            VirtualKey => &reference_locations.0.virtual_key,
            Layer => &reference_locations.0.layer,
            Template => &reference_locations.0.template,
            Include => unreachable!("includes can't be backreferenced"),
        };
        let locations: Option<Vec<_>> =
            location_map.0.get(&found_definition.ref_name).map(|spans| {
                spans
                    .iter()
                    .map(|span| GotoDefinitionLink {
                        source_range: found_definition.range,
                        target_range: lsp_range_from_span(span),
                        target_filename: span.file_name(),
                        kind: found_definition.ref_kind,
                        name: found_definition.ref_name.clone(),
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

#[derive(Debug, PartialEq)]
pub struct LocationInfoWithFilename {
    pub location_info: crate::helpers::LocationInfo,
    pub filename: String,
    pub is_definition: bool, // true if definition, false if reference
}

// Return all locations of symbols (without prefix character if present (like @ or $))
pub fn all_locations_of_symbol_at_pos(
    source_pos: &Position,
    source_doc: &Url,
    definition_locations_by_doc: &HashMap<Url, DefinitionLocations>,
    reference_locations_by_doc: &HashMap<Url, ReferenceLocations>,
    match_all_refs: bool, // Need to set `true` for workspace mode and `false` otherwise.
    path_to_url_fn: &dyn Fn(&str) -> anyhow::Result<Url>,
) -> Vec<LocationInfoWithFilename> {
    let i1 = goto_definition_for_token_at_pos(
        source_pos,
        source_doc,
        definition_locations_by_doc,
        reference_locations_by_doc,
        match_all_refs,
    )
    .map(|x| vec![x])
    .unwrap_or_default();

    let i2 = references_for_definition_at_pos(
        source_pos,
        source_doc,
        definition_locations_by_doc,
        reference_locations_by_doc,
        match_all_refs,
    )
    .unwrap_or_default();

    // Query for backreferences. Kinda hacky thing to do.

    let i1_rev = i1
        .first()
        .map(|x| {
            references_for_definition_at_pos(
                &x.target_range.start,
                &path_to_url_fn(&x.target_filename).unwrap(),
                definition_locations_by_doc,
                reference_locations_by_doc,
                match_all_refs,
            )
        })
        .unwrap_or_else(|| None)
        .unwrap_or_default();

    let i2_rev = i2
        .first()
        .map(|x| {
            goto_definition_for_token_at_pos(
                &x.target_range.start,
                &path_to_url_fn(&x.target_filename).unwrap(),
                definition_locations_by_doc,
                reference_locations_by_doc,
                match_all_refs,
            )
        })
        .unwrap_or_else(|| None)
        .map(|x| vec![x])
        .unwrap_or_default();

    let all = itertools::chain!(
        std::iter::zip(i1, repeat(true)),
        std::iter::zip(i2, repeat(false)),
        std::iter::zip(i1_rev, repeat(false)),
        std::iter::zip(i2_rev, repeat(true)),
    )
    .map(|(x, is_definition)| LocationInfoWithFilename {
        location_info: crate::helpers::LocationInfo {
            ref_kind: x.kind,
            ref_name: x.name,
            range: x.target_range,
        },
        filename: x.target_filename,
        is_definition,
    });

    all
    .dedup(). // FIXME: references_for_definition_at_pos gives duplicates, it needs fixing.
    collect::<Vec<_>>()
}
