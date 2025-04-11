use libafl::state::HasRand;
use libafl_bolts::rands::Rand;

use derive_new::new as New;

use super::{GenerationError, LspParamsGenerator};

use std::result::Result;

use crate::{
    lsp::HasPredefinedGenerators,
    lsp_input::LspInput,
    text_document::{
        grammar::tree_sitter::TreeIter,
        mutations::{TextDocumentSelector, text_document_selectors::RandomDoc},
    },
};

use std::marker::PhantomData;

use crate::text_document::{GrammarBasedMutation, TextDocument};

use lsp_types::{Position, Range, TextDocumentIdentifier};

#[derive(Debug)]
pub struct RangeInDoc(pub TextDocumentIdentifier, pub Range);

#[derive(Debug, New)]
pub struct RangeInDocGenerator<State, D> {
    pub(crate) range_selector: fn(&mut State, &TextDocument) -> Range,
    pub(crate) _phantom: PhantomData<D>,
}

impl<State, D> Clone for RangeInDocGenerator<State, D> {
    fn clone(&self) -> Self {
        Self::new(self.range_selector)
    }
}

impl<State, D> LspParamsGenerator<State> for RangeInDocGenerator<State, D>
where
    D: TextDocumentSelector<State>,
{
    type Output = RangeInDoc;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let range = (self.range_selector)(state, doc);
        let doc = TextDocumentIdentifier { uri };
        Ok(RangeInDoc(doc, range))
    }
}

impl<State> HasPredefinedGenerators<State> for RangeInDoc
where
    State: HasRand,
{
    type Generator = RangeInDocGenerator<State, RandomDoc>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: HasRand + 'static,
    {
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
        let whole_range = |_: &mut State, doc: &TextDocument| lsp_whole_range(doc);
        let random_range = |state: &mut State, doc: &TextDocument| {
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
        };
        let random_subtree = |state: &mut State, doc: &TextDocument| {
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
        };
        let after_range = |_: &mut State, doc: &TextDocument| {
            let Range { end, .. } = lsp_whole_range(doc);
            let start = end;
            let end = Position {
                line: 65536,
                character: end.character,
            };
            Range { start, end }
        };
        let inverted_range = |_: &mut State, doc: &TextDocument| {
            let Range { start, end } = lsp_whole_range(doc);
            Range {
                start: end,
                end: start,
            }
        };
        [
            whole_range,
            random_range,
            random_range,
            after_range,
            inverted_range,
            random_subtree,
        ]
        .map(RangeInDocGenerator::new)
    }
}
