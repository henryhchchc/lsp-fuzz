use std::{borrow::Cow, marker::PhantomData};

use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use libafl::{
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, tuples::NamedTuple, HasLen, Named};
use lsp_types::request::{
    Completion, GotoDefinition, HoverRequest, InlayHintRequest, SemanticTokensFullRequest,
};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

use crate::{
    lsp::{
        self,
        generation::{
            FullSemanticTokens, GoToDef, Hover, LspParamsGenerator, RequestInlayHint,
            TriggerCompletion,
        },
        LspMessage, Message, MessageParam,
    },
    macros::prop_mutator,
    mutators::SliceSwapMutator,
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
pub struct RandomPosition<const MAX: u32 = { u32::MAX }>;

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

#[derive(Debug)]
pub struct TerminalPosition;

impl<S> PositionSelector<S> for TerminalPosition
where
    S: HasRand,
{
    fn select_position(state: &mut S, doc: &TextDocument) -> Option<lsp_types::Position> {
        let range = state.rand_mut().choose(doc.terminal_ranges())?;
        let line = range.start_point.row as _;
        let character = range.start_point.column as _;
        Some(lsp_types::Position { line, character })
    }
}

pub type AddCompletion<S> =
    AppendMessage<Completion, S, TriggerCompletion<RandomDoc<S>, RandomPosition>>;

pub type RandomGotoDef<S> = AppendMessage<GotoDefinition, S, GoToDef<RandomDoc<S>, RandomPosition>>;
pub type InRangeGotoDef<S> =
    AppendMessage<GotoDefinition, S, GoToDef<RandomDoc<S>, TerminalPosition>>;

pub type RequestSemanticTokens<S> =
    AppendMessage<SemanticTokensFullRequest, S, FullSemanticTokens<RandomDoc<S>>>;

pub type RandomHover<S> = AppendMessage<HoverRequest, S, Hover<RandomDoc<S>, RandomPosition>>;

pub type AddInlayHints<S> = AppendMessage<InlayHintRequest, S, RequestInlayHint<RandomDoc<S>>>;

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

prop_mutator!(pub impl MessagesMutator for LspInput::messages type Vec<lsp::Message>);

pub type SwapRequests<S> = MessagesMutator<SliceSwapMutator<lsp::Message, S>>;

pub fn message_mutations<S>() -> impl MutatorsTuple<LspInput, S> + NamedTuple
where
    S: HasRand,
{
    tuple_list![
        RandomHover::new(),
        RequestSemanticTokens::new(),
        DropRandomMessage::new(),
        AddCompletion::new(),
        RandomGotoDef::new(),
        InRangeGotoDef::new(),
        AddInlayHints::new(),
        SwapRequests::new(SliceSwapMutator::new())
    ]
}
