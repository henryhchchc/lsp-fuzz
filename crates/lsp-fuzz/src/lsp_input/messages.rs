use std::{borrow::Cow, marker::PhantomData, num::NonZero, str::FromStr};

use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use libafl::{
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, tuples::NamedTuple, HasLen, Named};
use lsp_types::{
    request::{
        Completion, GotoDefinition, HoverRequest, InlayHintRequest, SemanticTokensFullRequest,
    },
    InlayHintParams, Position, Range, TextDocumentIdentifier, WorkDoneProgressParams,
};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

use crate::{
    lsp::{
        self,
        generation::{FullSemanticTokens, GoToDef, Hover, LspParamsGenerator, TriggerCompletion},
        LspMessage, Message, MessageParam,
    },
    text_document::{mutations::text_document_selectors::RandomDoc, TextDocument},
};

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

#[derive(Debug, New)]
pub struct AppendMessage<M, S, G> {
    _message: PhantomData<M>,
    _state: PhantomData<S>,
    _gen: PhantomData<G>,
}

impl<M, S, G> Named for AppendMessage<M, S, G> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("AppendMessage");
        &NAME
    }
}

impl<M, S, P, G> Mutator<LspInput, S> for AppendMessage<M, S, G>
where
    S: HasRand,
    M: LspMessage<Params = P>,
    P: MessageParam<M>,
    G: LspParamsGenerator<S, Result = P>,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        if let Some(params) = G::generate(state, input)? {
            let message = Message::from_params::<M>(params);
            input.messages.push(message);
            Ok(MutationResult::Mutated)
        } else {
            Ok(MutationResult::Skipped)
        }
    }
}

pub trait PositionSelector<S> {
    fn select_position(state: &mut S, doc: &TextDocument) -> Option<lsp_types::Position>;
}

#[derive(Debug)]
pub struct RandomPosition<const MAX: u32>;

impl<S, const MAX: u32> PositionSelector<S> for RandomPosition<MAX>
where
    S: HasRand,
{
    fn select_position(state: &mut S, _doc: &TextDocument) -> Option<lsp_types::Position> {
        let rand = state.rand_mut();
        let line = rand.between(0, MAX as _) as _;
        let character = rand.between(0, MAX as _) as _;
        Some(lsp_types::Position { line, character })
    }
}

pub type AddCompletion<S> =
    AppendMessage<Completion, S, TriggerCompletion<RandomDoc<S>, RandomPosition<100>>>;

pub type AddGotoDefinition<S> =
    AppendMessage<GotoDefinition, S, GoToDef<RandomDoc<S>, RandomPosition<100>>>;

pub type RequestSemanticTokens<S> =
    AppendMessage<SemanticTokensFullRequest, S, FullSemanticTokens<RandomDoc<S>>>;

pub type RandomHover<S> = AppendMessage<HoverRequest, S, Hover<RandomDoc<S>, RandomPosition<100>>>;

#[derive(Debug, New)]
pub struct AddInlayHints<S> {
    _state: PhantomData<S>,
}

impl<S> Named for AddInlayHints<S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("AddInlayHints");
        &NAME
    }
}

impl<S> Mutator<LspInput, S> for AddInlayHints<S>
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
        let uri = lsp_types::Uri::from_str("lsp-fuzz://main.c").unwrap();
        let start = {
            let line = rand.between(0, 1000) as _;
            let character = rand.between(0, 100) as _;
            Position { line, character }
        };
        let end = {
            let line = rand.between(0, 1000) as _;
            let character = rand.between(0, 100) as _;
            Position { line, character }
        };
        let inlay_hints = InlayHintParams {
            text_document: TextDocumentIdentifier { uri },
            range: Range { start, end },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let msg = Message::from_params::<InlayHintRequest>(inlay_hints);
        input.messages.push(msg);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug, New)]
pub struct DropRandomMessage<S> {
    _state: PhantomData<S>,
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

#[derive(Debug, New)]
pub struct SwapRequests<S> {
    _state: PhantomData<S>,
}

impl<S> Named for SwapRequests<S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("SwapRequests");
        &NAME
    }
}

impl<S> Mutator<LspInput, S> for SwapRequests<S>
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some(len) = NonZero::new(input.messages.len()) else {
            return Ok(MutationResult::Skipped);
        };
        if len < NonZero::new(2).unwrap() {
            return Ok(MutationResult::Skipped);
        }
        let rand = state.rand_mut();
        let idx1 = rand.below(len);
        let idx2 = rand.below(len);
        if idx1 != idx2 {
            input.messages.swap(idx1, idx2);
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
        RandomHover::new(),
        RequestSemanticTokens::new(),
        DropRandomMessage::new(),
        AddCompletion::new(),
        AddGotoDefinition::new(),
        AddInlayHints::new(),
        SwapRequests::new(),
    ]
}
