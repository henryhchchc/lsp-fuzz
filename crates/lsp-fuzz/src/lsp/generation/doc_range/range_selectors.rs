use itertools::Itertools;
use libafl::{
    HasMetadata,
    state::{HasCurrentTestcase, HasRand},
};
use libafl_bolts::rands::Rand;
use lsp_types::{Position, Range, Uri};

use crate::{
    lsp_input::{LspInput, server_response::metadata::LspResponseInfo},
    text_document::{GrammarBasedMutation, TextDocument, grammar::tree_sitter::TreeIter},
    utils::{ToLspRange, ToTreeSitterPoint},
};

fn lsp_whole_range(doc: &TextDocument) -> Range {
    let start = lsp_types::Position::default();
    let end = doc
        .lines()
        .enumerate()
        .last()
        .map(|(line_idx, line)| lsp_types::Position::new(line_idx as _, line.len() as _))
        .unwrap_or_default();
    lsp_types::Range::new(start, end)
}

pub(super) fn whole_range<State>(_: &mut State, _uri: &Uri, doc: &TextDocument) -> Range {
    lsp_whole_range(doc)
}

pub(super) fn random_valid_range<State: HasRand>(
    state: &mut State,
    _uri: &Uri,
    doc: &TextDocument,
) -> Range {
    let rand = state.rand_mut();
    let lines: Vec<_> = doc.lines().collect();
    let start_line_idx = rand.below_or_zero(lines.len());
    let start_line = lines[start_line_idx];
    let end_line_idx = rand.between(start_line_idx, lines.len() - 1);
    let end_line = lines[end_line_idx];
    let start = Position {
        line: start_line_idx as u32,
        character: rand.below_or_zero(start_line.len()) as u32,
    };
    let end = Position {
        line: end_line_idx as u32,
        character: rand.below_or_zero(end_line.len()) as u32,
    };
    Range { start, end }
}

pub(super) fn random_invalid_range<const MAX_RAND: usize, State: HasRand>(
    state: &mut State,
    _uri: &Uri,
    _doc: &TextDocument,
) -> Range {
    let rand = state.rand_mut();
    let start = Position {
        line: rand.below_or_zero(MAX_RAND) as u32,
        character: rand.below_or_zero(MAX_RAND) as u32,
    };
    let end = Position {
        line: rand.below_or_zero(MAX_RAND) as u32,
        character: rand.below_or_zero(MAX_RAND) as u32,
    };
    Range { start, end }
}

pub(super) fn subtree_node_type<State: HasRand>(
    state: &mut State,
    _uri: &Uri,
    doc: &TextDocument,
) -> Range {
    let subtree_types = doc.parse_tree().iter().into_group_map_by(|it| it.kind_id());
    if let Some((_kind, subtrees)) = state.rand_mut().choose(subtree_types)
        && let Some(subtree) = state.rand_mut().choose(subtrees)
    {
        subtree.range().to_lsp_range()
    } else {
        lsp_whole_range(doc)
    }
}

pub(super) fn diagnosed_range<State: HasRand + HasCurrentTestcase<LspInput>>(
    state: &mut State,
    uri: &Uri,
    doc: &TextDocument,
) -> Range {
    let mut select = || -> Option<Range> {
        let test_case = state.current_testcase().ok()?;
        let response_info = test_case.metadata::<LspResponseInfo>().ok()?;
        let ranges: Vec<_> = response_info
            .diagnostics
            .iter()
            .filter(|it| &it.uri == uri)
            .map(|it| it.range)
            .collect();
        drop(test_case);
        let rand = state.rand_mut();
        rand.choose(ranges)
    };

    if let Some(range) = select() {
        range
    } else {
        subtree_node_type(state, uri, doc)
    }
}

pub(super) fn diagnosed_parent<State: HasRand + HasCurrentTestcase<LspInput>>(
    state: &mut State,
    uri: &Uri,
    doc: &TextDocument,
) -> Range {
    let mut select = || -> Option<Range> {
        let test_case = state.current_testcase().ok()?;
        let response_info = test_case.metadata::<LspResponseInfo>().ok()?;
        let ranges: Vec<_> = response_info
            .diagnostics
            .iter()
            .filter(|it| &it.uri == uri)
            .map(|it| it.range)
            .collect();
        drop(test_case);
        let rand = state.rand_mut();
        let range = rand.choose(ranges)?;
        let node = doc
            .parse_tree()
            .root_node()
            .descendant_for_point_range(range.start.to_ts_point(), range.end.to_ts_point())?;
        Some(node.parent()?.range().to_lsp_range())
    };

    if let Some(range) = select() {
        range
    } else {
        subtree_node_type(state, uri, doc)
    }
}

pub(super) fn symbols_range<State: HasRand + HasCurrentTestcase<LspInput>>(
    state: &mut State,
    uri: &Uri,
    doc: &TextDocument,
) -> Range {
    let mut select = || -> Option<Range> {
        let test_case = state.current_testcase().ok()?;
        let response_info = test_case.metadata::<LspResponseInfo>().ok()?;
        let ranges: Vec<_> = response_info
            .symbol_ranges
            .iter()
            .filter(|it| &it.uri == uri)
            .map(|it| it.range)
            .collect();
        drop(test_case);
        let rand = state.rand_mut();
        rand.choose(ranges)
    };

    if let Some(range) = select() {
        range
    } else {
        subtree_node_type(state, uri, doc)
    }
}

pub(super) fn after_range<State>(_: &mut State, _: &Uri, doc: &TextDocument) -> Range {
    let Range { end, .. } = lsp_whole_range(doc);
    let start = end;
    let end = Position {
        line: 65536,
        character: end.character,
    };
    Range { start, end }
}

pub(super) fn inverted_range<State>(_: &mut State, _: &Uri, doc: &TextDocument) -> Range {
    let Range { start, end } = lsp_whole_range(doc);
    Range {
        start: end,
        end: start,
    }
}
