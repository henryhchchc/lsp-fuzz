use std::{marker::PhantomData, result::Result, str::FromStr};

use derive_new::new as New;
use libafl::state::HasRand;
use lsp_types::TextDocumentIdentifier;

use super::{DynGenerator, GenerationError, LspParamsGenerator, boxed_generator};
use crate::{
    lsp::HasGenerators,
    lsp_input::LspInput,
    text_document::mutations::{core::TextDocumentSelector, text_document_selectors::RandomDoc},
    utils::generate_random_uri_content,
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
pub struct RandomVirtualDocumentIdentifierGenerator;

impl<State> LspParamsGenerator<State> for RandomVirtualDocumentIdentifierGenerator
where
    State: HasRand,
{
    type Output = TextDocumentIdentifier;

    fn generate(
        &self,
        state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let uri_content = generate_random_uri_content(state.rand_mut(), 256);
        let uri = lsp_types::Uri::from(
            fluent_uri::Uri::from_str(&format!("lsp-fuzz://{uri_content}")).unwrap(),
        );
        Ok(Self::Output { uri })
    }
}

impl<State> HasGenerators<State> for TextDocumentIdentifier
where
    State: HasRand,
{
    type Generator = DynGenerator<State, TextDocumentIdentifier>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        let mut generators: Vec<Self::Generator> = Vec::new();
        if config.use_context() {
            generators.push(boxed_generator(
                TextDocumentIdentifierGenerator::<RandomDoc>::new(),
            ));
        } else {
            generators.push(boxed_generator(
                RandomVirtualDocumentIdentifierGenerator::new(),
            ));
        }
        generators
    }
}
