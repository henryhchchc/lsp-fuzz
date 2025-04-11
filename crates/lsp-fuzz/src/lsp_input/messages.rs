use std::{
    any::type_name,
    borrow::Cow,
    fmt::Debug,
    iter::{once, repeat},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use libafl::{
    HasMetadata,
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{
    HasLen, Named,
    rands::Rand,
    tuples::{Merge, NamedTuple},
};
use lsp_fuzz_grammars::WELL_KNOWN_HIGHLIGHT_CAPTURE_NAMES;
use serde::{Deserialize, Serialize};
use trait_gen::trait_gen;
use tuple_list::{tuple_list, tuple_list_type};

use crate::{
    lsp::{
        self, ClientToServerMessage, HasPredefinedGenerators, LspMessage, MessageParam,
        generation::{DefaultGenerator, GenerationError, LspParamsGenerator, MappingGenerator},
    },
    macros::{append_randoms, prop_mutator},
    mutators::SliceSwapMutator,
    text_document::{
        TextDocument,
        grammar::tree_sitter::{CapturesIterator, TSNodeExt, TreeIter},
    },
    utils::RandExt,
};

use super::LspInput;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Deref, DerefMut)]
pub struct LspMessages {
    inner: Vec<lsp::ClientToServerMessage>,
}

impl HasLen for LspMessages {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

pub trait PositionSelector<State> {
    fn select_position(&self, state: &mut State, doc: &TextDocument)
    -> Option<lsp_types::Position>;
}

#[derive(Debug, New)]
pub struct RandomPosition {
    rand_max: usize,
}

impl<State> PositionSelector<State> for RandomPosition
where
    State: HasRand,
{
    fn select_position(
        &self,
        state: &mut State,
        _doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let rand = state.rand_mut();
        let line = rand.between(0, self.rand_max) as _;
        let character = rand.between(0, self.rand_max) as _;
        Some(lsp_types::Position { line, character })
    }
}

#[derive(Debug, New)]
pub struct ValidPosition;

impl<State> PositionSelector<State> for ValidPosition
where
    State: HasRand,
{
    fn select_position(
        &self,
        state: &mut State,
        doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let (index, line) = state.rand_mut().choose(doc.lines().enumerate())?;
        let character = state.rand_mut().choose(0..line.len())?;
        Some(lsp_types::Position {
            line: index as _,
            character: character as _,
        })
    }
}

#[derive(Debug, Clone, Copy, New)]
pub struct TerminalStartPosition;

impl<State> PositionSelector<State> for TerminalStartPosition
where
    State: HasRand,
{
    fn select_position(
        &self,
        state: &mut State,
        doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let terminals = doc
            .metadata()
            .parse_tree
            .iter()
            .filter(|it| it.child_count() == 0);
        let range = state.rand_mut().choose(terminals)?;
        Some(range.lsp_start_position())
    }
}

#[derive(Debug, Clone, Copy, New)]
pub struct HighlightSteer;

#[derive(Debug, Serialize, Deserialize, Deref, DerefMut, libafl_bolts::SerdeAny)]
pub struct HighlightGroupUsageMetadata {
    #[deref]
    #[deref_mut]
    inner: ahash::HashMap<String, usize>,
}

impl HighlightGroupUsageMetadata {
    pub fn new<Names, Name>(highlight_group_names: Names) -> Self
    where
        Names: IntoIterator<Item = Name>,
        Name: Into<String>,
    {
        let inner = highlight_group_names
            .into_iter()
            .map(Into::into)
            .zip(repeat(0))
            .collect();
        Self { inner }
    }
}

impl<State> PositionSelector<State> for HighlightSteer
where
    State: HasRand + HasMetadata,
{
    fn select_position(
        &self,
        state: &mut State,
        doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let usage_stats = state.metadata_or_insert_with(|| {
            HighlightGroupUsageMetadata::new(WELL_KNOWN_HIGHLIGHT_CAPTURE_NAMES)
        });
        let max_usage = usage_stats.values().copied().max().unwrap_or_default();
        let weights: Vec<_> = usage_stats
            .iter()
            .map(|(name, &usage)| (name.clone(), max_usage - usage))
            .collect();
        let chosen = state.rand_mut().weighted_choose(weights)?;
        let captures = CapturesIterator::new(doc, &chosen)?;
        let node = state.rand_mut().choose(captures)?;
        let pos = node.lsp_start_position();
        let usage_stats = state
            .metadata_mut::<HighlightGroupUsageMetadata>()
            .expect("We ensured it is inserted");
        let usage = usage_stats
            .get_mut(&chosen)
            .expect("The entry is in the map");
        *usage += 1;
        Some(pos)
    }
}

#[derive(Debug, New)]
pub struct DropRandomMessage<State> {
    _state: PhantomData<State>,
}

impl<State> Named for DropRandomMessage<State> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("DropRandomMessage");
        &NAME
    }
}

impl<State> Mutator<LspInput, State> for DropRandomMessage<State>
where
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
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

prop_mutator!(pub impl MessagesMutator for LspInput::messages type Vec<lsp::ClientToServerMessage>);

pub type SwapRequests<State> = MessagesMutator<SliceSwapMutator<lsp::ClientToServerMessage, State>>;

use lsp_types::*;

#[trait_gen(P ->
        ApplyWorkspaceEditParams,
        CallHierarchyIncomingCallsParams,
        CallHierarchyOutgoingCallsParams,
        CancelParams,
        CodeAction,
        CodeLens,
        CompletionItem,
        ConfigurationParams,
        CreateFilesParams,
        DeleteFilesParams,
        DidChangeConfigurationParams,
        DidChangeNotebookDocumentParams,
        DidChangeTextDocumentParams,
        DidChangeWatchedFilesParams,
        DidChangeWorkspaceFoldersParams,
        DidCloseNotebookDocumentParams,
        DidCloseTextDocumentParams,
        DidOpenNotebookDocumentParams,
        DidSaveNotebookDocumentParams,
        DidSaveTextDocumentParams,
        DocumentLink,
        ExecuteCommandParams,
        InlayHint,
        InlineValueParams,
        PublishDiagnosticsParams,
        RegistrationParams,
        RenameFilesParams,
        SemanticTokensDeltaParams,
        TypeHierarchySubtypesParams,
        TypeHierarchySupertypesParams,
        UnregistrationParams,
        WillSaveTextDocumentParams,
        WorkspaceSymbol,
)]
impl<State> HasPredefinedGenerators<State> for P {
    type Generator = Arc<dyn LspParamsGenerator<State, Output = Self>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        []
    }
}

#[trait_gen(P ->
    WorkDoneProgressParams,
    PartialResultParams,
    (),
    serde_json::Map<String, serde_json::Value>,
    serde_json::Value,
)]
impl<State: 'static> HasPredefinedGenerators<State> for P {
    type Generator = DefaultGenerator<Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        [DefaultGenerator::new()]
    }
}


impl<State, A, B> HasPredefinedGenerators<State> for OneOf<A, B>
where
    State: 'static,
    A: HasPredefinedGenerators<State> + 'static,
    B: HasPredefinedGenerators<State> + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<State, Output = Self>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        let left_gen = A::generators()
            .into_iter()
            .map(|g| Rc::new(MappingGenerator::new(g, OneOf::Left)) as _);
        let right_gen = B::generators()
            .into_iter()
            .map(|g| Rc::new(MappingGenerator::new(g, OneOf::Right)) as _);
        left_gen.chain(right_gen)
    }
}

#[derive(Debug, New)]
pub struct OptionGenerator<State, T>
where
    State: 'static,
    T: HasPredefinedGenerators<State>,
{
    inner: Option<T::Generator>,
    _state: PhantomData<State>,
}

impl<State, T> Clone for OptionGenerator<State, T>
where
    State: 'static,
    T: HasPredefinedGenerators<State>,
    T::Generator: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl<State, T> LspParamsGenerator<State> for OptionGenerator<State, T>
where
    State: 'static,
    T: HasPredefinedGenerators<State> + 'static,
{
    type Output = Option<T>;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        if let Some(ref inner) = self.inner {
            Ok(Some(inner.generate(state, input)?))
        } else {
            Ok(None)
        }
    }
}

impl<State, T> HasPredefinedGenerators<State> for Option<T>
where
    State: 'static,
    T: HasPredefinedGenerators<State> + 'static,
    T::Generator: Clone,
{
    type Generator = OptionGenerator<State, T>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        T::generators()
            .into_iter()
            .flat_map(|g| [OptionGenerator::new(Some(g))])
            .chain(once(OptionGenerator::new(None)))
    }
}

#[derive(Debug, Clone)]
pub struct VecGenerator<G, const MAX_ITEMS: usize = 5> {
    element_generators: Vec<G>,
}

impl<G, const MAX_ITEMS: usize> VecGenerator<G, MAX_ITEMS> {
    pub fn new(element_generators: impl IntoIterator<Item = G>) -> Self {
        let element_generators = element_generators.into_iter().collect();
        Self { element_generators }
    }
}

impl<State, G, const MAX_ITEMS: usize> LspParamsGenerator<State> for VecGenerator<G, MAX_ITEMS>
where
    G: LspParamsGenerator<State>,
    State: HasRand,
{
    type Output = Vec<G::Output>;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let len = state.rand_mut().between(1, MAX_ITEMS);
        let mut items = Vec::with_capacity(len);
        let mut anything_generated = false;
        for _ in 0..len {
            if let Some(generator) = state.rand_mut().choose(&self.element_generators) {
                match generator.generate(state, input) {
                    Ok(item) => {
                        items.push(item);
                        anything_generated = true;
                    }
                    Err(GenerationError::NothingGenerated) => {}
                    Err(e) => return Err(e),
                }
            }
        }
        anything_generated
            .then_some(items)
            .ok_or(GenerationError::NothingGenerated)
    }
}

impl<State, T> HasPredefinedGenerators<State> for Vec<T>
where
    State: HasRand + 'static,
    T: HasPredefinedGenerators<State> + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<State, Output = Vec<T>>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        let vec_generator: Self::Generator =
            Rc::new(VecGenerator::<T::Generator, 5>::new(T::generators()));
        let default_generator: Self::Generator = Rc::new(DefaultGenerator::new());
        [vec_generator, default_generator]
    }
}

pub struct AppendRandomlyGeneratedMessage<M, State>
where
    M: LspMessage,
    M::Params: HasPredefinedGenerators<State>,
{
    name: Cow<'static, str>,
    generators: Vec<<M::Params as HasPredefinedGenerators<State>>::Generator>,
}

impl<M: LspMessage, State> Debug for AppendRandomlyGeneratedMessage<M, State>
where
    M::Params: HasPredefinedGenerators<State>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let generators_desc = format!(
            "{} {}",
            self.generators.len(),
            type_name::<<M::Params as HasPredefinedGenerators<State>>::Generator>()
        );
        f.debug_struct("AppendRandomlyGeneratedMessage")
            .field("generators", &generators_desc)
            .finish()
    }
}

impl<M, State: 'static> AppendRandomlyGeneratedMessage<M, State>
where
    M: LspMessage,
    M::Params: HasPredefinedGenerators<State>,
{
    pub fn with_predefined() -> Self {
        let name = Cow::Owned(format!("AppendRandomlyGenerated {}", M::METHOD));
        let generators: Vec<_> = M::Params::generators().into_iter().collect();
        assert!(!generators.is_empty(), "No generators for {}", M::METHOD);
        Self { name, generators }
    }
}

impl<M, State> Named for AppendRandomlyGeneratedMessage<M, State>
where
    M: LspMessage,
    M::Params: HasPredefinedGenerators<State>,
{
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<M, State, P> Mutator<LspInput, State> for AppendRandomlyGeneratedMessage<M, State>
where
    State: HasRand,
    M: LspMessage<Params = P>,
    M::Params: HasPredefinedGenerators<State>,
    P: MessageParam<M>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some(generator) = state.rand_mut().choose(&self.generators) else {
            return Ok(MutationResult::Skipped);
        };
        let params = match generator.generate(state, input) {
            Ok(params) => params,
            Err(GenerationError::NothingGenerated) => return Ok(MutationResult::Skipped),
            Err(GenerationError::Error(e)) => return Err(e),
        };
        let message = ClientToServerMessage::from_params::<M>(params);
        input.messages.push(message);
        Ok(MutationResult::Mutated)
    }
}

append_randoms! {

    /// Mutation operators for each message type with `AppendRandomlyGeneratedMessage` mutator.
   fn append_randomly_generated_messages() -> AppendRandomlyGenerateMessageMutations {
        // request::CallHierarchyIncomingCalls,
        // request::CallHierarchyOutgoingCalls,
        // request::CodeActionResolveRequest,
        // request::CodeLensResolve,
        // request::DocumentLinkResolve,
        // request::ExecuteCommand,
        // request::InlayHintResolveRequest,
        // request::InlineValueRefreshRequest,
        // request::InlineValueRequest,
        request::OnTypeFormatting,
        request::RangeFormatting,
        // request::ResolveCompletionItem,
        request::SelectionRangeRequest,
        // request::SemanticTokensFullDeltaRequest,
        // request::TypeHierarchySubtypes,
        // request::TypeHierarchySupertypes,
        // request::WillCreateFiles,
        // request::WillDeleteFiles,
        // request::WillRenameFiles,
        // request::WillSaveWaitUntil,
        // request::WorkspaceSymbolResolve,
        request::CallHierarchyPrepare,
        request::CodeActionRequest,
        request::CodeLensRequest,
        request::ColorPresentationRequest,
        request::Completion,
        request::DocumentColor,
        request::DocumentDiagnosticRequest,
        request::DocumentHighlightRequest,
        request::DocumentLinkRequest,
        request::DocumentSymbolRequest,
        request::FoldingRangeRequest,
        request::GotoDeclaration,
        request::GotoDefinition,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::HoverRequest,
        request::InlayHintRequest,
        request::LinkedEditingRange,
        request::MonikerRequest,
        request::PrepareRenameRequest,
        request::References,
        request::Rename,
        request::SemanticTokensFullRequest,
        request::SemanticTokensRangeRequest,
        request::SignatureHelpRequest,
        request::TypeHierarchyPrepare,
        request::WorkspaceDiagnosticRefresh,
        request::WorkspaceDiagnosticRequest,
        request::WorkspaceSymbolRequest,
        // notification::Cancel,
        // notification::DidChangeConfiguration,
        // notification::DidChangeNotebookDocument,
        // notification::DidChangeTextDocument,
        // notification::DidChangeWatchedFiles,
        // notification::DidChangeWorkspaceFolders,
        // notification::DidCloseNotebookDocument,
        // notification::DidCloseTextDocument,
        // notification::DidCreateFiles,
        // notification::DidDeleteFiles,
        // notification::DidOpenNotebookDocument,
        // notification::DidRenameFiles,
        // notification::DidSaveNotebookDocument,
        // notification::DidSaveTextDocument,
        // notification::WillSaveTextDocument,
        // notification::WorkDoneProgressCancel,
        notification::LogTrace,
        notification::SetTrace,
    }
}

pub fn message_mutations<State>() -> impl MutatorsTuple<LspInput, State> + NamedTuple
where
    State: HasRand + HasMetadata + 'static,
{
    let swap = tuple_list![SwapRequests::new(SliceSwapMutator::new())];
    append_randomly_generated_messages()
        .merge(swap)
        .merge(message_reductions())
}

pub fn message_reductions<State>() -> tuple_list_type![DropRandomMessage<State>]
where
    State: HasRand,
{
    tuple_list![DropRandomMessage::new()]
}
