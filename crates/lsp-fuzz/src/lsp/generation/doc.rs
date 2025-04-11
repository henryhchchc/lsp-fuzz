use super::GenerationError;

use std::result::Result;

use crate::{
    lsp::HasPredefinedGenerators, lsp_input::LspInput,
    text_document::mutations::text_document_selectors::RandomDoc,
};

use libafl::state::HasRand;
use lsp_types::TextDocumentIdentifier;

use crate::text_document::mutations::TextDocumentSelector;

use super::LspParamsGenerator;
use derive_new::new as New;

use std::marker::PhantomData;

#[derive(Debug, New)]
pub struct TextDocumentIdentifierGenerator<D> {
    pub(crate) _phantom: PhantomData<D>,
}

impl<T> Clone for TextDocumentIdentifierGenerator<T> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<State, D> LspParamsGenerator<State> for TextDocumentIdentifierGenerator<D>
where
    D: TextDocumentSelector<State>,
{
    type Output = TextDocumentIdentifier;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, _) = D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output { uri })
    }
}

impl<State> HasPredefinedGenerators<State> for TextDocumentIdentifier
where
    State: HasRand + 'static,
{
    type Generator = TextDocumentIdentifierGenerator<RandomDoc>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        [TextDocumentIdentifierGenerator::<RandomDoc>::new()]
    }
}
