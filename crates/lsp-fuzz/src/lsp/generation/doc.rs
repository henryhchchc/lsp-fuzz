use std::{marker::PhantomData, rc::Rc, result::Result, str::FromStr};

use derive_new::new as New;
use libafl::state::HasRand;
use libafl_bolts::rands::Rand;
use lsp_types::TextDocumentIdentifier;

use super::{GenerationError, LspParamsGenerator};
use crate::{
    lsp::HasPredefinedGenerators,
    lsp_input::LspInput,
    text_document::mutations::{core::TextDocumentSelector, text_document_selectors::RandomDoc},
};

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

#[derive(Debug, New)]
pub struct MeaninglessTextDocumentIdentifierGenerator;

impl<State> LspParamsGenerator<State> for MeaninglessTextDocumentIdentifierGenerator
where
    State: HasRand,
{
    type Output = TextDocumentIdentifier;

    fn generate(
        &self,
        state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let uri_content = state.rand_mut().below_or_zero(65536);
        let uri = lsp_types::Uri::from(
            fluent_uri::Uri::from_str(&format!("lsp-fuzz://{uri_content}")).unwrap(),
        );
        Ok(Self::Output { uri })
    }
}

impl<State> HasPredefinedGenerators<State> for TextDocumentIdentifier
where
    State: HasRand,
{
    type Generator = Rc<dyn LspParamsGenerator<State, Output = TextDocumentIdentifier>>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        let mut generators: Vec<Self::Generator> = Vec::new();
        if config.ctx_awareness {
            generators.push(Rc::new(TextDocumentIdentifierGenerator::<RandomDoc>::new()));
        } else {
            generators.push(Rc::new(MeaninglessTextDocumentIdentifierGenerator::new()));
        }
        generators
    }
}
