use std::{borrow::Cow, marker::PhantomData};

use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use libafl::{
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, tuples::NamedTuple, HasLen, Named};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

use crate::{
    lsp::{self, generation::LspParamsGenerator, LspMessage, Message, MessageParam},
    macros::prop_mutator,
    mutators::SliceSwapMutator,
    text_document::TextDocument,
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
pub struct RandomPosition<const MAX: u32 = 1024>;

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
pub struct TerminalStartPosition;

impl<S> PositionSelector<S> for TerminalStartPosition
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

pub mod append_mutations {

    use lsp_types::request::{
        DocumentHighlightRequest, GotoDeclaration, GotoDefinition, GotoImplementation,
        GotoTypeDefinition, HoverRequest, InlayHintRequest, References, SemanticTokensFullRequest,
        TypeHierarchyPrepare,
    };

    use crate::{
        lsp::generation::{
            self, FindReferences, FullSemanticTokens, GoToDef, Hover, InlayHintWholdDoc,
            TriggerCompletion, TypeHierarchyPrep,
        },
        text_document::mutations::text_document_selectors::RandomDoc,
    };

    use super::AppendMessage;

    pub type Completion<P, S> =
        AppendMessage<lsp_types::request::Completion, S, TriggerCompletion<RandomDoc<S>, P>>;

    pub type GotoDecl<P, S> = AppendMessage<GotoDeclaration, S, GoToDef<RandomDoc<S>, P>>;
    pub type GotoDef<P, S> = AppendMessage<GotoDefinition, S, GoToDef<RandomDoc<S>, P>>;
    pub type GotoTypeDef<P, S> = AppendMessage<GotoTypeDefinition, S, GoToDef<RandomDoc<S>, P>>;
    pub type GotoImpl<P, S> = AppendMessage<GotoImplementation, S, GoToDef<RandomDoc<S>, P>>;
    pub type FindRef<P, S> = AppendMessage<References, S, FindReferences<RandomDoc<S>, P>>;
    pub type PrepTypeHierarchy<P, S> =
        AppendMessage<TypeHierarchyPrepare, S, TypeHierarchyPrep<RandomDoc<S>, P>>;
    pub type DocumentHighlight<P, S> =
        AppendMessage<DocumentHighlightRequest, S, generation::DocumentHighlight<RandomDoc<S>, P>>;

    pub type SemanticTokensFull<S> =
        AppendMessage<SemanticTokensFullRequest, S, FullSemanticTokens<RandomDoc<S>>>;

    pub type PerformHover<P, S> = AppendMessage<HoverRequest, S, Hover<RandomDoc<S>, P>>;

    pub type InlayHints<S> = AppendMessage<InlayHintRequest, S, InlayHintWholdDoc<RandomDoc<S>>>;
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

prop_mutator!(pub impl MessagesMutator for LspInput::messages type Vec<lsp::Message>);

pub type SwapRequests<S> = MessagesMutator<SliceSwapMutator<lsp::Message, S>>;

pub fn message_mutations<S>() -> impl MutatorsTuple<LspInput, S> + NamedTuple
where
    S: HasRand,
{
    tuple_list![
        append_mutations::GotoDef::<RandomPosition, _>::new(),
        append_mutations::GotoDef::<TerminalStartPosition, _>::new(),
        append_mutations::GotoDecl::<RandomPosition, _>::new(),
        append_mutations::GotoDecl::<TerminalStartPosition, _>::new(),
        append_mutations::GotoTypeDef::<RandomPosition, _>::new(),
        append_mutations::GotoTypeDef::<TerminalStartPosition, _>::new(),
        append_mutations::GotoImpl::<RandomPosition, _>::new(),
        append_mutations::GotoImpl::<TerminalStartPosition, _>::new(),
        append_mutations::FindRef::<RandomPosition, _>::new(),
        append_mutations::FindRef::<TerminalStartPosition, _>::new(),
        append_mutations::PrepTypeHierarchy::<RandomPosition, _>::new(),
        append_mutations::PrepTypeHierarchy::<TerminalStartPosition, _>::new(),
        append_mutations::DocumentHighlight::<RandomPosition, _>::new(),
        append_mutations::DocumentHighlight::<TerminalStartPosition, _>::new(),
        append_mutations::PerformHover::<RandomPosition, _>::new(),
        append_mutations::PerformHover::<TerminalStartPosition, _>::new(),
        append_mutations::SemanticTokensFull::new(),
        append_mutations::Completion::<RandomPosition, _>::new(),
        append_mutations::InlayHints::new(),
        SwapRequests::new(SliceSwapMutator::new())
    ]
}

pub fn message_reductions<S>() -> impl MutatorsTuple<LspInput, S> + NamedTuple
where
    S: HasRand,
{
    tuple_list![DropRandomMessage::new()]
}
