use std::{borrow::Cow, marker::PhantomData, str::FromStr};

use libafl::{
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, tuples::NamedTuple, HasLen, Named};
use lsp_types::{
    HoverParams, PartialResultParams, Position, SemanticTokensParams, TextDocumentIdentifier,
    TextDocumentPositionParams, WorkDoneProgressParams,
};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

use crate::lsp;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct LspMessages {
    pub messages: Vec<lsp::Message>,
}

impl HasLen for LspMessages {
    fn len(&self) -> usize {
        self.messages.len()
    }
}

#[derive(Debug)]
pub struct RandomHover<S> {
    _state: PhantomData<S>,
}

impl<S> Default for RandomHover<S> {
    fn default() -> Self {
        Self {
            _state: PhantomData,
        }
    }
}

impl<S> Named for RandomHover<S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("RandomHover");
        &NAME
    }
}

impl<S> Mutator<LspMessages, S> for RandomHover<S>
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspMessages,
    ) -> Result<MutationResult, libafl::Error> {
        if input.messages.len() > 10 {
            return Ok(MutationResult::Skipped);
        }
        let rand = state.rand_mut();
        let document_uri = lsp_types::Uri::from_str("workspace://main.c").unwrap();
        let line = rand.between(0, 100) as _;
        let character = rand.between(0, 100) as _;
        let hover = lsp::Message::HoverRequest(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: document_uri },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        });
        input.messages.push(hover);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug)]
pub struct RequestSemanticTokens<S> {
    _state: PhantomData<S>,
}

impl<S> Default for RequestSemanticTokens<S> {
    fn default() -> Self {
        Self {
            _state: PhantomData,
        }
    }
}

impl<S> Named for RequestSemanticTokens<S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("SemanticTokens");
        &NAME
    }
}

impl<S> Mutator<LspMessages, S> for RequestSemanticTokens<S>
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        _state: &mut S,
        input: &mut LspMessages,
    ) -> Result<MutationResult, libafl::Error> {
        if input.messages.len() > 10 {
            return Ok(MutationResult::Skipped);
        }
        let document_uri = lsp_types::Uri::from_str("workspace://main.c").unwrap();
        let text_document = TextDocumentIdentifier { uri: document_uri };
        let semantic_tokens = lsp::Message::SemanticTokensFullRequest(SemanticTokensParams {
            text_document,
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        });
        input.messages.push(semantic_tokens);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug)]
pub struct DropRandomMessage<S> {
    _state: PhantomData<S>,
}

impl<S> Default for DropRandomMessage<S> {
    fn default() -> Self {
        Self {
            _state: PhantomData,
        }
    }
}

impl<S> Named for DropRandomMessage<S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("DropRandomMessage");
        &NAME
    }
}

impl<S> Mutator<LspMessages, S> for DropRandomMessage<S>
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspMessages,
    ) -> Result<MutationResult, libafl::Error> {
        let rand = state.rand_mut();
        if let Some(index) = rand.choose(0..input.messages.len()) {
            input.messages.remove(index);
            Ok(MutationResult::Mutated)
        } else {
            Ok(MutationResult::Skipped)
        }
    }
}

pub fn message_mutations<S>() -> impl MutatorsTuple<LspMessages, S> + NamedTuple
where
    S: HasRand,
{
    tuple_list![
        RandomHover::default(),
        RequestSemanticTokens::default(),
        DropRandomMessage::default()
    ]
}
