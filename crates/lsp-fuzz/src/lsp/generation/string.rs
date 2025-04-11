use std::{marker::PhantomData, num::NonZeroUsize, result::Result};

use libafl::{HasMetadata, state::HasRand};
use libafl_bolts::rands::Rand;

use super::{GenerationError, LspParamsGenerator};
use crate::{
    lsp::{HasPredefinedGenerators, generation::meta::DefaultGenerator},
    lsp_input::LspInput,
    text_document::{
        GrammarBasedMutation,
        grammar::tree_sitter::TreeIter,
        mutations::{TextDocumentSelector, text_document_selectors::RandomDoc},
    },
    utf8::UTF8Tokens,
};

#[derive(Debug, Default)]
pub struct UTF8TokensGenerator;

impl UTF8TokensGenerator {
    pub const fn new() -> Self {
        Self
    }
}

impl<State> LspParamsGenerator<State> for UTF8TokensGenerator
where
    State: HasMetadata + HasRand,
{
    type Output = String;

    fn generate(
        &self,
        state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let token_cnt = state
            .metadata()
            .map(UTF8Tokens::len)
            .ok()
            .and_then(NonZeroUsize::new)
            .ok_or(GenerationError::NothingGenerated)?;
        let idx = state.rand_mut().below(token_cnt);
        // SAFETY: We checked just now that the metadata is present
        let tokens: &UTF8Tokens = unsafe { state.metadata().unwrap_unchecked() };
        let token = tokens[idx].clone();
        Ok(token)
    }
}

#[derive(Debug, Default)]
pub struct TerminalTextGenerator<DocSel> {
    pub(crate) _phantom: PhantomData<DocSel>,
}

impl<DocSel> TerminalTextGenerator<DocSel> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<State, DocSel> LspParamsGenerator<State> for TerminalTextGenerator<DocSel>
where
    State: HasRand,
    DocSel: TextDocumentSelector<State>,
{
    type Output = String;
    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let doc = DocSel::select_document(state, input)
            .map(|it| it.1)
            .ok_or(GenerationError::NothingGenerated)?;
        let terminal_text = doc
            .parse_tree()
            .iter()
            .filter(|it| it.child_count() == 0)
            .filter_map(|node| node.utf8_text(doc.content()).ok());
        let text = state
            .rand_mut()
            .choose(terminal_text)
            .ok_or(GenerationError::NothingGenerated)?;

        Ok(text.to_owned())
    }
}

impl<State> HasPredefinedGenerators<State> for String
where
    State: HasRand + HasMetadata + 'static,
{
    type Generator = &'static dyn LspParamsGenerator<State, Output = Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        static DEFAULT: DefaultGenerator<String> = DefaultGenerator::new();
        static TOKENS: UTF8TokensGenerator = UTF8TokensGenerator::new();
        static TERMINAL_TEXT: TerminalTextGenerator<RandomDoc> = TerminalTextGenerator::new();
        [&DEFAULT as _, &TOKENS as _, &TERMINAL_TEXT as _]
    }
}
