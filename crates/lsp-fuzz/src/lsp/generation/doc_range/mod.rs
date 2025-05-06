use std::{marker::PhantomData, result::Result};

use derive_new::new as New;
use libafl::state::HasRand;
use lsp_types::{Range, TextDocumentIdentifier};

use super::{GenerationError, LspParamsGenerator};
use crate::{
    lsp::HasPredefinedGenerators,
    lsp_input::LspInput,
    text_document::{
        TextDocument,
        mutations::{core::TextDocumentSelector, text_document_selectors::RandomDoc},
    },
};

mod range_selectors;

#[derive(Debug)]
pub struct Selection(pub TextDocumentIdentifier, pub Range);

#[derive(Debug, New)]
pub struct RangeInDocGenerator<State, D> {
    range_selector: fn(&mut State, &TextDocument) -> Range,
    _phantom: PhantomData<D>,
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
    type Output = Selection;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let range = (self.range_selector)(state, doc);
        let doc = TextDocumentIdentifier { uri };
        Ok(Selection(doc, range))
    }
}

impl<State> HasPredefinedGenerators<State> for Selection
where
    State: HasRand,
{
    type Generator = RangeInDocGenerator<State, RandomDoc>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator>
    where
        State: HasRand,
    {
        let range_sels = if config.invalid_ranges {
            [
                range_selectors::whole_range,
                range_selectors::random_range,
                range_selectors::random_range,
                range_selectors::after_range,
                range_selectors::inverted_range,
                range_selectors::random_subtree,
            ]
        } else {
            [
                range_selectors::whole_range,
                range_selectors::random_range,
                range_selectors::random_range,
                range_selectors::random_range,
                range_selectors::random_range,
                range_selectors::random_subtree,
            ]
        };
        range_sels.map(RangeInDocGenerator::new)
    }
}
