use std::{borrow::Cow, fmt::Debug, marker::PhantomData, rc::Rc};

use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use libafl::{
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
    HasMetadata,
};
use libafl_bolts::{
    rands::Rand,
    tuples::{Merge, NamedTuple},
    HasLen, Named,
};
use serde::{Deserialize, Serialize};
use trait_gen::trait_gen;
use tuple_list::{tuple_list, tuple_list_type};

use crate::{
    lsp::{
        self,
        generation::{
            ConstGenerator, DefaultGenerator, GenerationError, LspParamsGenerator,
            MappingGenerator, TextDocumentIdentifierGenerator, TextDocumentPositionParamsGenerator,
            TokensGenerator,
        },
        HasPredefinedGenerators, LspMessage, Message, MessageParam,
    },
    macros::{append_randoms, prop_mutator},
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

use lsp_types::*;

#[trait_gen(P ->
        ApplyWorkspaceEditParams,
        CallHierarchyIncomingCallsParams,
        CallHierarchyOutgoingCallsParams,
        CancelParams,
        CodeAction,
        CodeLens,
        ColorPresentationParams,
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
        DocumentFormattingParams,
        DocumentLink,
        DocumentOnTypeFormattingParams,
        DocumentRangeFormattingParams,
        ExecuteCommandParams,
        FoldingRangeParams,
        InlayHint,
        InlineValueParams,
        LogTraceParams,
        MonikerParams,
        PublishDiagnosticsParams,
        RegistrationParams,
        RenameFilesParams,
        RenameParams,
        SelectionRangeParams,
        SemanticTokensDeltaParams,
        SignatureHelpParams,
        TypeHierarchySubtypesParams,
        TypeHierarchySupertypesParams,
        UnregistrationParams,
        WillSaveTextDocumentParams,
        WorkDoneProgressCancelParams,
        WorkDoneProgressCreateParams,
        WorkspaceDiagnosticParams,
        WorkspaceSymbol,
)]
impl<S> HasPredefinedGenerators<S> for P {
    type Generator = Box<dyn LspParamsGenerator<S, Output = Self>>;

    fn generators() -> Vec<Self::Generator> {
        vec![]
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
    type Generator = DefaultGenerator<S, Self>;

    fn generators() -> Vec<Self::Generator> {
        vec![DefaultGenerator::new()]
    }
}

impl<S> HasPredefinedGenerators<S> for bool
where
    S: HasRand + 'static,
{
    type Generator = ConstGenerator<Self>;

    fn generators() -> Vec<Self::Generator> {
        vec![ConstGenerator::new(false), ConstGenerator::new(true)]
    }
}

impl<S> HasPredefinedGenerators<S> for TextDocumentIdentifier
where
    S: HasRand + 'static,
{
    type Generator = TextDocumentIdentifierGenerator<S, RandomDoc<S>>;

    fn generators() -> Vec<Self::Generator> {
        vec![TextDocumentIdentifierGenerator::<S, RandomDoc<S>>::new()]
    }
}

impl<S> HasPredefinedGenerators<S> for TextDocumentPositionParams
where
    S: HasRand + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<S, Output = Self>>;

    fn generators() -> Vec<Self::Generator> {
        vec![
            Rc::new(TextDocumentPositionParamsGenerator::<
                S,
                RandomDoc<S>,
                RandomPosition,
            >::new()),
            Rc::new(TextDocumentPositionParamsGenerator::<
                S,
                RandomDoc<S>,
                TerminalStartPosition,
            >::new()),
        ]
    }
}

impl<S> HasPredefinedGenerators<S> for SetTraceParams {
    type Generator = ConstGenerator<Self>;

    fn generators() -> Vec<Self::Generator>
    where
        S: 'static,
    {
        [TraceValue::Messages, TraceValue::Off, TraceValue::Verbose]
            .into_iter()
            .map(|value| Self { value })
            .map(ConstGenerator::new)
            .collect()
    }
}

impl<S, A, B> HasPredefinedGenerators<S> for OneOf<A, B>
where
    S: 'static,
    A: HasPredefinedGenerators<S> + 'static,
    B: HasPredefinedGenerators<S> + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<S, Output = Self>>;

    fn generators() -> Vec<Self::Generator> {
        let left_gen = A::generators()
            .into_iter()
            .map(|g| Rc::new(MappingGenerator::new(g, OneOf::Left)) as _);
        let right_gen = B::generators()
            .into_iter()
            .map(|g| Rc::new(MappingGenerator::new(g, OneOf::Right)) as _);
        left_gen.chain(right_gen).collect()
    }
}

#[derive(Debug, New)]
pub struct OptionGenerator<S, T>
where
    S: 'static,
    T: HasPredefinedGenerators<S> + 'static,
{
    inner: Option<T::Generator>,
    _state: PhantomData<S>,
}

impl<S, T> Clone for OptionGenerator<S, T>
where
    S: 'static,
    T: HasPredefinedGenerators<S> + 'static,
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
{
    type Generator = OptionGenerator<S, T>;

    fn generators() -> Vec<Self::Generator> {
        T::generators()
            .into_iter()
            .flat_map(|g| [OptionGenerator::new(None), OptionGenerator::new(Some(g))])
            .collect()
    }
}

impl<S> HasPredefinedGenerators<S> for String
where
    S: HasRand + HasMetadata + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<S, Output = Self>>;

    fn generators() -> Vec<Self::Generator> {
        vec![
            Rc::new(DefaultGenerator::new()) as _,
            Rc::new(TokensGenerator::<Self>::new()) as _,
        ]
    }
}

impl<S, T> HasPredefinedGenerators<S> for Vec<T>
where
    S: 'static,
    T: HasPredefinedGenerators<S> + 'static,
{
    type Generator = DefaultGenerator<S, Self>;

    fn generators() -> Vec<Self::Generator> {
        vec![DefaultGenerator::new()]
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
        f.debug_struct("AppendRandomlyGeneratedMessage")
            .field(
                "generators",
                &format!("({} dyn generators)", self.generators.len()),
            )
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
        let generators = M::Params::generators();
        assert_ne!(generators.len(), 0, "No generators for {}", M::METHOD);
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
        let message = Message::from_params::<M>(params);
        input.messages.push(message);
        Ok(MutationResult::Mutated)
    }
}

append_randoms! {

    /// Mutation operators for each message type with `AppendRandomlyGeneratedMessage` mutator.
    pub fn append_randomly_generated_messages() -> AppendRandomlyGenerateMessageMutations {
        // request::CallHierarchyIncomingCalls,
        // request::CallHierarchyOutgoingCalls,
        request::CodeActionRequest,
        // request::CodeActionResolveRequest,
        // request::CodeLensResolve,
        // request::ColorPresentationRequest,
        // request::DocumentLinkResolve,
        // request::ExecuteCommand,
        // request::FoldingRangeRequest,
        // request::InlayHintResolveRequest,
        // request::InlineValueRefreshRequest,
        // request::InlineValueRequest,
        // request::MonikerRequest,
        // request::OnTypeFormatting,
        request::PrepareRenameRequest,
        // request::RangeFormatting,
        // request::Rename,
        // request::ResolveCompletionItem,
        // request::SelectionRangeRequest,
        // request::SemanticTokensFullDeltaRequest,
        // request::SignatureHelpRequest,
        // request::TypeHierarchySubtypes,
        // request::TypeHierarchySupertypes,
        // request::WillCreateFiles,
        // request::WillDeleteFiles,
        // request::WillRenameFiles,
        // request::WillSaveWaitUntil,
        // request::WorkspaceDiagnosticRefresh,
        // request::WorkspaceDiagnosticRequest,
        // request::WorkspaceSymbolResolve,
        request::CallHierarchyPrepare,
        request::CodeLensRequest,
        request::Completion,
        request::DocumentColor,
        request::DocumentDiagnosticRequest,
        request::DocumentHighlightRequest,
        request::DocumentLinkRequest,
        request::DocumentSymbolRequest,
        request::GotoDeclaration,
        request::GotoDefinition,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::HoverRequest,
        request::InlayHintRequest,
        request::LinkedEditingRange,
        request::References,
        request::SemanticTokensFullRequest,
        request::SemanticTokensRangeRequest,
        request::TypeHierarchyPrepare,
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
