use std::{marker::PhantomData, rc::Rc};

use libafl::{HasMetadata, state::HasRand};
use lsp_types::{TextDocumentIdentifier, TextDocumentPositionParams};

use super::{GenerationError, HasPredefinedGenerators, LspParamsGenerator};
use crate::{
    lsp_input::{
        LspInput,
        messages::{
            HighlightSteer, PositionSelector, RandomPosition, TerminalStartPosition, ValidPosition,
        },
    },
    text_document::mutations::{TextDocumentSelector, text_document_selectors::RandomDoc},
};

#[derive(Debug)]
pub struct TextDocumentPositionParamsGenerator<D, PosSel> {
    position_selector: PosSel,
    _phantom: PhantomData<D>,
}

impl<D, PosSel> TextDocumentPositionParamsGenerator<D, PosSel> {
    pub const fn new(position_selector: PosSel) -> Self {
        Self {
            position_selector,
            _phantom: PhantomData,
        }
    }
}

impl<State, D, PosSel> LspParamsGenerator<State> for TextDocumentPositionParamsGenerator<D, PosSel>
where
    D: TextDocumentSelector<State>,
    PosSel: PositionSelector<State>,
{
    type Output = lsp_types::TextDocumentPositionParams;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let position = self
            .position_selector
            .select_position(state, doc)
            .ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output {
            text_document: TextDocumentIdentifier { uri },
            position,
        })
    }
}

impl<State> HasPredefinedGenerators<State> for TextDocumentPositionParams
where
    State: HasRand + HasMetadata,
{
    type Generator = Rc<dyn LspParamsGenerator<State, Output = Self>>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        type SelectInRandomDoc<PosSel> = TextDocumentPositionParamsGenerator<RandomDoc, PosSel>;
        let term_start_pos = TerminalStartPosition::new();
        let term_start: Self::Generator = Rc::new(SelectInRandomDoc::new(term_start_pos));
        let steer: Self::Generator = Rc::new(SelectInRandomDoc::new(HighlightSteer::new()));

        let mut generators = Vec::new();
        generators.extend([
            Rc::new(SelectInRandomDoc::new(ValidPosition::new())),
            term_start.clone(),
            term_start.clone(),
            term_start.clone(),
            steer.clone(),
            steer.clone(),
            steer.clone(),
        ]);
        if config.invalid_positions {
            generators.push(Rc::new(SelectInRandomDoc::new(RandomPosition::new(1024))));
        }
        generators
    }
}
