use libafl::state::HasRand;
use libafl_bolts::rands::Rand;
use lsp_types::{Position, Range};

use crate::text_document::{GrammarBasedMutation, TextDocument, grammar::tree_sitter::TreeIter};

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

pub(super) fn whole_range<State>(_: &mut State, doc: &TextDocument) -> Range {
    lsp_whole_range(doc)
}

pub(super) fn random_range<State: HasRand>(state: &mut State, doc: &TextDocument) -> Range {
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

pub(super) fn random_subtree<State: HasRand>(state: &mut State, doc: &TextDocument) -> Range {
    let tree_iter = doc.parse_tree().iter();
    if let Some(node) = state.rand_mut().choose(tree_iter) {
        let start = node.start_position();
        let start = Position {
            line: start.row as u32,
            character: start.column as u32,
        };
        let end = node.end_position();
        let end = Position {
            line: end.row as u32,
            character: end.column as u32,
        };
        Range { start, end }
    } else {
        lsp_whole_range(doc)
    }
}

pub(super) fn after_range<State>(_: &mut State, doc: &TextDocument) -> Range {
    let Range { end, .. } = lsp_whole_range(doc);
    let start = end;
    let end = Position {
        line: 65536,
        character: end.character,
    };
    Range { start, end }
}

pub(super) fn inverted_range<State>(_: &mut State, doc: &TextDocument) -> Range {
    let Range { start, end } = lsp_whole_range(doc);
    Range {
        start: end,
        end: start,
    }
}
