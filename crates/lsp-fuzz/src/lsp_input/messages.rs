use std::{
    any::type_name, borrow::Cow, fmt::Debug, iter::once, marker::PhantomData, rc::Rc, sync::Arc,
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
use serde::{Deserialize, Serialize};
use trait_gen::trait_gen;
use tuple_list::{tuple_list, tuple_list_type};

use crate::{
    lsp::{
        self, ClientToServerMessage, HasPredefinedGenerators, LspMessage, MessageParam,
        generation::{
            ConstGenerator, DefaultGenerator, GenerationError, LspParamsGenerator,
            MappingGenerator, TextDocumentIdentifierGenerator, TextDocumentPositionParamsGenerator,
            TokensGenerator,
        },
    },
    macros::{append_randoms, prop_mutator},
    mutators::SliceSwapMutator,
    text_document::{TextDocument, mutations::text_document_selectors::RandomDoc},
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
pub struct ValidPosition;

impl<S> PositionSelector<S> for ValidPosition
where
    S: HasRand,
{
    fn select_position(state: &mut S, doc: &TextDocument) -> Option<lsp_types::Position> {
        let (index, line) = state.rand_mut().choose(doc.lines().enumerate())?;
        let character = state.rand_mut().choose(0..line.len())?;
        Some(lsp_types::Position {
            line: index as _,
            character: character as _,
        })
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

prop_mutator!(pub impl MessagesMutator for LspInput::messages type Vec<lsp::ClientToServerMessage>);

pub type SwapRequests<S> = MessagesMutator<SliceSwapMutator<lsp::ClientToServerMessage, S>>;

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
impl<S> HasPredefinedGenerators<S> for P {
    type Generator = Arc<dyn LspParamsGenerator<S, Output = Self>>;

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
impl<S: 'static> HasPredefinedGenerators<S> for P {
    type Generator = DefaultGenerator<Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        [DefaultGenerator::new()]
    }
}

impl<S> HasPredefinedGenerators<S> for bool
where
    S: HasRand + 'static,
{
    type Generator = ConstGenerator<Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        [ConstGenerator::new(false), ConstGenerator::new(true)]
    }
}

impl<S> HasPredefinedGenerators<S> for TextDocumentIdentifier
where
    S: HasRand + 'static,
{
    type Generator = TextDocumentIdentifierGenerator<RandomDoc<S>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        [TextDocumentIdentifierGenerator::<RandomDoc<S>>::new()]
    }
}

impl<S> HasPredefinedGenerators<S> for TextDocumentPositionParams
where
    S: HasRand + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<S, Output = Self>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        let term_start: Self::Generator = Rc::new(TextDocumentPositionParamsGenerator::<
            RandomDoc<S>,
            TerminalStartPosition,
        >::new());
        let result: [Self::Generator; 6] = [
            Rc::new(TextDocumentPositionParamsGenerator::<
                RandomDoc<S>,
                ValidPosition,
            >::new()),
            Rc::new(TextDocumentPositionParamsGenerator::<
                RandomDoc<S>,
                RandomPosition,
            >::new()),
            Rc::new(TextDocumentPositionParamsGenerator::<
                RandomDoc<S>,
                TerminalStartPosition,
            >::new()),
            term_start.clone(),
            term_start.clone(),
            term_start.clone(),
        ];
        result
    }
}

impl<S, A, B> HasPredefinedGenerators<S> for OneOf<A, B>
where
    S: 'static,
    A: HasPredefinedGenerators<S> + 'static,
    B: HasPredefinedGenerators<S> + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<S, Output = Self>>;

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
pub struct OptionGenerator<S, T>
where
    S: 'static,
    T: HasPredefinedGenerators<S>,
{
    inner: Option<T::Generator>,
    _state: PhantomData<S>,
}

impl<S, T> Clone for OptionGenerator<S, T>
where
    S: 'static,
    T: HasPredefinedGenerators<S>,
    T::Generator: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl<S, T> LspParamsGenerator<S> for OptionGenerator<S, T>
where
    S: 'static,
    T: HasPredefinedGenerators<S> + 'static,
{
    type Output = Option<T>;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        if let Some(ref inner) = self.inner {
            Ok(Some(inner.generate(state, input)?))
        } else {
            Ok(None)
        }
    }
}

impl<S, T> HasPredefinedGenerators<S> for Option<T>
where
    S: 'static,
    T: HasPredefinedGenerators<S> + 'static,
    T::Generator: Clone,
{
    type Generator = OptionGenerator<S, T>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        T::generators()
            .into_iter()
            .flat_map(|g| [OptionGenerator::new(Some(g))])
            .chain(once(OptionGenerator::new(None)))
    }
}

impl<S> HasPredefinedGenerators<S> for String
where
    S: HasRand + HasMetadata + 'static,
{
    type Generator = &'static dyn LspParamsGenerator<S, Output = Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        static DEFAULT: DefaultGenerator<String> = DefaultGenerator::new();
        static TOKENS: TokensGenerator<String> = TokensGenerator::new();
        [&DEFAULT as _, &TOKENS as _]
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

impl<S, G, const MAX_ITEMS: usize> LspParamsGenerator<S> for VecGenerator<G, MAX_ITEMS>
where
    G: LspParamsGenerator<S>,
    S: HasRand,
{
    type Output = Vec<G::Output>;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
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

impl<S, T> HasPredefinedGenerators<S> for Vec<T>
where
    S: HasRand + 'static,
    T: HasPredefinedGenerators<S> + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<S, Output = Vec<T>>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        let vec_generator: Self::Generator =
            Rc::new(VecGenerator::<T::Generator, 5>::new(T::generators()));
        let default_generator: Self::Generator = Rc::new(DefaultGenerator::new());
        [vec_generator, default_generator]
    }
}

pub struct AppendRandomlyGeneratedMessage<M, S>
where
    M: LspMessage,
    M::Params: HasPredefinedGenerators<S>,
{
    name: Cow<'static, str>,
    generators: Vec<<M::Params as HasPredefinedGenerators<S>>::Generator>,
}

impl<M: LspMessage, S> Debug for AppendRandomlyGeneratedMessage<M, S>
where
    M::Params: HasPredefinedGenerators<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let generators_desc = format!(
            "{} {}",
            self.generators.len(),
            type_name::<<M::Params as HasPredefinedGenerators<S>>::Generator>()
        );
        f.debug_struct("AppendRandomlyGeneratedMessage")
            .field("generators", &generators_desc)
            .finish()
    }
}

impl<M, S: 'static> AppendRandomlyGeneratedMessage<M, S>
where
    M: LspMessage,
    M::Params: HasPredefinedGenerators<S>,
{
    pub fn with_predefined() -> Self {
        let name = Cow::Owned(format!("AppendRandomlyGenerated {}", M::METHOD));
        let generators: Vec<_> = M::Params::generators().into_iter().collect();
        assert!(!generators.is_empty(), "No generators for {}", M::METHOD);
        Self { name, generators }
    }
}

impl<M, S> Named for AppendRandomlyGeneratedMessage<M, S>
where
    M: LspMessage,
    M::Params: HasPredefinedGenerators<S>,
{
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<M, S, P> Mutator<LspInput, S> for AppendRandomlyGeneratedMessage<M, S>
where
    S: HasRand,
    M: LspMessage<Params = P>,
    M::Params: HasPredefinedGenerators<S>,
    P: MessageParam<M>,
{
    fn mutate(
        &mut self,
        state: &mut S,
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

pub fn message_mutations<S>() -> impl MutatorsTuple<LspInput, S> + NamedTuple
where
    S: HasRand + HasMetadata + 'static,
{
    let swap = tuple_list![SwapRequests::new(SliceSwapMutator::new())];
    append_randomly_generated_messages()
        .merge(swap)
        .merge(message_reductions())
}

pub fn message_reductions<S>() -> tuple_list_type![DropRandomMessage<S>]
where
    S: HasRand,
{
    tuple_list![DropRandomMessage::new()]
}
