use std::{borrow::Cow, marker::PhantomData, str::FromStr};

use derive_more::derive::{Deref, DerefMut};
use libafl::{
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, tuples::NamedTuple, HasLen, Named};
use lsp_types::{
    request::SemanticTokensFullRequest, HoverParams, Position, TextDocumentIdentifier,
    TextDocumentPositionParams, WorkDoneProgressParams,
};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

use crate::lsp::{self, IntoMessage, LspMessage, LspParamsGen};

use super::LspInput;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Deref, DerefMut)]
pub struct LspMessages {
    inner: Vec<lsp::Message>,
}

impl HasLen for LspMessages {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[derive(Debug)]
pub struct AppendMessage<M, S> {
    _message: PhantomData<M>,
    _state: PhantomData<S>,
}

impl<M, S> Default for AppendMessage<M, S> {
    fn default() -> Self {
        Self {
            _message: PhantomData,
            _state: PhantomData,
        }
    }
}

impl<M, S> Named for AppendMessage<M, S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("AppendMessage");
        &NAME
    }
}

impl<M, S, PG> Mutator<LspInput, S> for AppendMessage<M, S>
where
    S: HasRand,
    M: LspMessage<Params = PG> + IntoMessage<M>,
    PG: LspParamsGen,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let param = PG::generate_one::<S>(state, input);
        let message = M::into_message(param);
        input.messages.push(message);
        Ok(MutationResult::Mutated)
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

impl<S> Mutator<LspInput, S> for RandomHover<S>
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
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

pub type RequestSemanticTokens<S> = AppendMessage<SemanticTokensFullRequest, S>;

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

impl<S> Mutator<LspInput, S> for DropRandomMessage<S>
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
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

pub fn message_mutations<S>() -> impl MutatorsTuple<LspInput, S> + NamedTuple
where
    S: HasRand,
{
    tuple_list![
        RandomHover::default(),
        RequestSemanticTokens::default(),
        DropRandomMessage::default()
    ]
}
