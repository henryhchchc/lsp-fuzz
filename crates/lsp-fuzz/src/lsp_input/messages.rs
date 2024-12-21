use std::{borrow::Cow, fmt::Debug, iter::once, marker::PhantomData, rc::Rc};

use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use libafl::{
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, tuples::NamedTuple, HasLen, Named};
use serde::{Deserialize, Serialize};
use trait_gen::trait_gen;
use tuple_list::tuple_list;

use crate::{
    lsp::{
        self,
        generation::{
            ConstGenerator, DefaultGenerator, GenerationError, LspParamsGenerator,
            TextDocumentIdentifierGenerator, TextDocumentPositionParamsGenerator,
        },
        LspMessage, Message, MessageParam,
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

pub trait HasPredefinedGenerators<S> {
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>>
    where
        S: 'static;
}

use lsp_types::*;

#[trait_gen(P ->
        ApplyWorkspaceEditParams,
        CallHierarchyIncomingCallsParams,
        CallHierarchyOutgoingCallsParams,
        CancelParams,
        CodeAction,
        CodeActionParams,
        CodeLens,
        ColorPresentationParams,
        CompletionItem,
        CompletionParams,
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
        InlayHintParams,
        InlineValueParams,
        LinkedEditingRangeParams,
        LogMessageParams,
        LogTraceParams,
        MonikerParams,
        PublishDiagnosticsParams,
        RegistrationParams,
        RenameFilesParams,
        RenameParams,
        SelectionRangeParams,
        SemanticTokensDeltaParams,
        SemanticTokensRangeParams,
        SetTraceParams,
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
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
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
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
        vec![Rc::new(DefaultGenerator::new())]
    }
}

impl<S> HasPredefinedGenerators<S> for bool
where
    S: HasRand + 'static,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
        vec![
            Rc::new(ConstGenerator::new(false)),
            Rc::new(ConstGenerator::new(true)),
        ]
    }
}

impl<S> HasPredefinedGenerators<S> for TextDocumentIdentifier
where
    S: HasRand + 'static,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
        vec![Rc::new(
            TextDocumentIdentifierGenerator::<S, RandomDoc<S>>::new(),
        )]
    }
}

impl<S> HasPredefinedGenerators<S> for TextDocumentPositionParams
where
    S: HasRand + 'static,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
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

impl<S, A, B> HasPredefinedGenerators<S> for OneOf<A, B>
where
    A: HasPredefinedGenerators<S>,
    B: HasPredefinedGenerators<S>,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
        vec![]
    }
}

impl<S, T> HasPredefinedGenerators<S> for Option<T>
where
    S: 'static,
    T: HasPredefinedGenerators<S> + 'static,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
        T::generators()
            .into_iter()
            .map(|g| Rc::new(g.map(Some)) as _)
            .chain(once(Rc::new(DefaultGenerator::new()) as _))
            .collect()
    }
}

impl<S> HasPredefinedGenerators<S> for String
where
    S: HasRand + 'static,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
        vec![Rc::new(DefaultGenerator::new())]
    }
}

impl<S, T> HasPredefinedGenerators<S> for Vec<T>
where
    S: 'static,
    T: HasPredefinedGenerators<S> + 'static,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>> {
        vec![Rc::new(DefaultGenerator::new())]
    }
}

pub struct AppendRandomlyGeneratedMessage<M: LspMessage, S> {
    generators: Vec<Rc<dyn LspParamsGenerator<S, Output = M::Params>>>,
}

impl<M: LspMessage, S> Debug for AppendRandomlyGeneratedMessage<M, S> {
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
    <M as LspMessage>::Params: HasPredefinedGenerators<S>,
{
    pub fn with_predefined() -> Self {
        let generators = M::Params::generators();
        Self { generators }
    }
}

impl<M, S> Named for AppendRandomlyGeneratedMessage<M, S>
where
    M: LspMessage,
{
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("AppendRandomlyGeneratedMessage");
        &NAME
    }
}

impl<M, S, P> Mutator<LspInput, S> for AppendRandomlyGeneratedMessage<M, S>
where
    S: HasRand,
    M: LspMessage<Params = P>,
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

pub fn message_mutations<S>() -> impl MutatorsTuple<LspInput, S> + NamedTuple
where
    S: HasRand + 'static,
{
    append_randoms![
        request::WorkspaceSymbolRequest,
        request::WorkspaceSymbolResolve,
        request::ExecuteCommand,
        request::WillSaveWaitUntil,
        request::Completion,
        request::ResolveCompletionItem,
        request::HoverRequest,
        request::SignatureHelpRequest,
        request::GotoDeclaration,
        request::GotoDefinition,
        request::References,
        request::DocumentHighlightRequest,
        request::DocumentSymbolRequest,
        request::CodeActionRequest,
        request::CodeLensRequest,
        request::CodeLensResolve,
        request::DocumentLinkRequest,
        request::DocumentLinkResolve,
        request::RangeFormatting,
        request::OnTypeFormatting,
        request::Rename,
        request::DocumentColor,
        request::ColorPresentationRequest,
        request::FoldingRangeRequest,
        request::PrepareRenameRequest,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::SelectionRangeRequest,
        request::CallHierarchyIncomingCalls,
        request::CallHierarchyOutgoingCalls,
        request::MonikerRequest,
        request::LinkedEditingRange,
        request::CallHierarchyPrepare,
        request::TypeHierarchyPrepare,
        request::SemanticTokensFullRequest,
        request::SemanticTokensFullDeltaRequest,
        request::SemanticTokensRangeRequest,
        request::InlayHintRequest,
        request::InlineValueRequest,
        request::DocumentDiagnosticRequest,
        request::WorkspaceDiagnosticRequest,
        request::WorkspaceDiagnosticRefresh,
        request::TypeHierarchySupertypes,
        request::TypeHierarchySubtypes,
        request::WillCreateFiles,
        request::WillRenameFiles,
        request::WillDeleteFiles,
        request::InlineValueRefreshRequest,
        request::CodeActionResolveRequest,
        request::InlayHintResolveRequest,
        notification::Cancel,
        notification::SetTrace,
        notification::LogTrace,
        notification::WorkDoneProgressCancel,
        notification::DidChangeTextDocument,
        notification::WillSaveTextDocument,
        notification::DidSaveTextDocument,
        notification::DidCloseTextDocument,
        notification::DidOpenNotebookDocument,
        notification::DidChangeNotebookDocument,
        notification::DidSaveNotebookDocument,
        notification::DidCloseNotebookDocument,
        notification::DidChangeConfiguration,
        notification::DidChangeWatchedFiles,
        notification::DidChangeWorkspaceFolders,
        notification::DidCreateFiles,
        notification::DidRenameFiles,
        notification::DidDeleteFiles
    ]
}

pub fn message_reductions<S>() -> impl MutatorsTuple<LspInput, S> + NamedTuple
where
    S: HasRand,
{
    tuple_list![DropRandomMessage::new()]
}
