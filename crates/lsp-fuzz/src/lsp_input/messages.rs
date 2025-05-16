use std::{any::type_name, borrow::Cow, fmt::Debug, iter::repeat, marker::PhantomData, mem};

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

use super::LspInput;
use crate::{
    lsp::{
        self, GeneratorsConfig, HasPredefinedGenerators, LspMessage, LspMessageMeta, MessageParam,
        code_context::CodeContextRef,
        generation::{GenerationError, LspParamsGenerator, meta::DefaultGenerator},
        json_rpc::MessageId,
    },
    macros::{append_randoms, prop_mutator},
    mutators::SliceSwapMutator,
    text_document::{
        TextDocument,
        grammar::tree_sitter::{CapturesIterator, TSNodeExt, TreeIter},
    },
    utils::RandExt,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Deref, DerefMut)]
pub struct LspMessageSequence {
    inner: Vec<lsp::LspMessage>,
}

#[derive(Debug)]
pub struct EnumMessages<'a> {
    next_id: usize,
    messages: <&'a Vec<lsp::LspMessage> as IntoIterator>::IntoIter,
}

impl<'a> Iterator for EnumMessages<'a> {
    type Item = (Option<MessageId>, &'a lsp::LspMessage);

    fn next(&mut self) -> Option<Self::Item> {
        match self.messages.next() {
            Some(msg) if msg.is_request() => {
                let new_id = self.next_id + 1;
                let id = mem::replace(&mut self.next_id, new_id);
                Some((Some(MessageId::Number(id)), msg))
            }
            Some(msg) if msg.is_notification() => Some((None, msg)),
            None => None,
            _ => unreachable!(),
        }
    }
}

impl LspMessageSequence {
    pub fn enumerate_messages(&self) -> EnumMessages<'_> {
        EnumMessages {
            next_id: 0,
            messages: self.inner.iter(),
        }
    }

    pub fn calibrate(&mut self, doc_uri: &Uri, input_edit: tree_sitter::InputEdit) {
        self.inner
            .iter_mut()
            .filter(|it| it.document().is_some_and(|it| &it.uri == doc_uri))
            .for_each(|message| calibrate_message(message, input_edit));
    }
}

fn calibrate_message(message: &mut LspMessage, input_edit: tree_sitter::InputEdit) {
    // Helper function to determine if a position is after the edit
    fn is_after_edit(pos: &lsp_types::Position, edit: &tree_sitter::InputEdit) -> bool {
        (pos.line as usize)
            .cmp(&edit.old_end_position.row)
            .then_with(|| (pos.character as usize).cmp(&edit.old_end_position.column))
            .is_ge()
    }

    // Helper function to update a position if it's after the edit
    fn update_position(pos: &mut lsp_types::Position, edit: &tree_sitter::InputEdit) {
        if is_after_edit(pos, edit) {
            let line_diff = edit.new_end_position.row as i64 - edit.old_end_position.row as i64;
            let col_diff =
                edit.new_end_position.column as i64 - edit.old_end_position.column as i64;
            pos.line = (pos.line as i64 + line_diff) as u32;
            pos.character = (pos.character as i64 + col_diff) as u32;
        }
    }

    if let Some(pos) = message.position_mut() {
        update_position(pos, &input_edit);
    } else if let Some(range) = message.range_mut() {
        update_position(&mut range.start, &input_edit);
        update_position(&mut range.end, &input_edit);
    }
}

impl HasLen for LspMessageSequence {
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

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

prop_mutator!(pub impl MessagesMutator for LspInput::messages type Vec<lsp::LspMessage>);

pub type SwapRequests<State> = MessagesMutator<SliceSwapMutator<lsp::LspMessage, State>>;

use lsp_types::*;

#[trait_gen(P ->
    WorkDoneProgressParams,
    PartialResultParams,
    (),
    serde_json::Map<String, serde_json::Value>,
    serde_json::Value,
)]
impl<State: 'static> HasPredefinedGenerators<State> for P {
    type Generator = DefaultGenerator<Self>;

    fn generators(
        _config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        [DefaultGenerator::new()]
    }
}

#[derive(Debug, Clone)]
pub struct VecGenerator<G> {
    element_generators: Vec<G>,
    max_items: usize,
}

impl<G> VecGenerator<G> {
    pub fn new(element_generators: impl IntoIterator<Item = G>, max_items: usize) -> Self {
        let element_generators = element_generators.into_iter().collect();
        Self {
            element_generators,
            max_items,
        }
    }
}

impl<State, G> LspParamsGenerator<State> for VecGenerator<G>
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
        let len = state.rand_mut().below_or_zero(self.max_items);
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
    State: HasRand,
    T: HasPredefinedGenerators<State>,
{
    type Generator = VecGenerator<T::Generator>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        [VecGenerator::<T::Generator>::new(T::generators(config), 5)]
    }
}

pub struct AppendRandomlyGeneratedMessage<M, State>
where
    M: LspMessageMeta,
    M::Params: HasPredefinedGenerators<State>,
{
    name: Cow<'static, str>,
    generators: Vec<<M::Params as HasPredefinedGenerators<State>>::Generator>,
}

impl<M: LspMessageMeta, State> Debug for AppendRandomlyGeneratedMessage<M, State>
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

pub const MAX_MESSAGES: usize = 10;

impl<M, State> AppendRandomlyGeneratedMessage<M, State>
where
    M: LspMessageMeta,
    M::Params: HasPredefinedGenerators<State>,
{
    pub fn with_predefined(config: &GeneratorsConfig) -> Self {
        let name = Cow::Owned(format!("AppendRandomlyGenerated {}", M::METHOD));
        let generators: Vec<_> = M::Params::generators(config).into_iter().collect();
        assert!(!generators.is_empty(), "No generators for {}", M::METHOD);
        Self { name, generators }
    }
}

impl<M, State> Named for AppendRandomlyGeneratedMessage<M, State>
where
    M: LspMessageMeta,
    M::Params: HasPredefinedGenerators<State>,
{
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<M, State> Mutator<LspInput, State> for AppendRandomlyGeneratedMessage<M, State>
where
    State: HasRand,
    M: LspMessageMeta,
    M::Params: HasPredefinedGenerators<State> + MessageParam<M>,
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
        let message = LspMessage::from_params::<M>(params);
        if input.messages.len() >= MAX_MESSAGES {
            let being_replaced = state.rand_mut().choose(input.messages.iter_mut()).expect(
                "There must be at least one message in the input when entering this branch",
            );
            *being_replaced = message;
        } else {
            input.messages.push(message);
        }
        Ok(MutationResult::Mutated)
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

append_randoms! {

    /// Mutation operators for each message type with `AppendRandomlyGeneratedMessage` mutator.
   fn append_randomly_generated_messages(config: &GeneratorsConfig) -> AppendRandomlyGenerateMessageMutations {
        // request::CallHierarchyIncomingCalls,
        // request::CallHierarchyOutgoingCalls,
        // request::CodeActionResolveRequest,
        // request::CodeLensResolve,
        // request::DocumentLinkResolve,
        // request::ExecuteCommand,
        // request::InlayHintResolveRequest,
        // request::InlineValueRefreshRequest,
        // request::InlineValueRequest,
        // request::ResolveCompletionItem,
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
        request::Formatting,
        request::GotoDeclaration,
        request::GotoDefinition,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::HoverRequest,
        request::InlayHintRequest,
        request::LinkedEditingRange,
        request::MonikerRequest,
        request::OnTypeFormatting,
        request::PrepareRenameRequest,
        request::RangeFormatting,
        request::References,
        request::Rename,
        request::SelectionRangeRequest,
        request::SemanticTokensFullDeltaRequest,
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

pub fn message_mutations<State>(
    config: &GeneratorsConfig,
) -> impl MutatorsTuple<LspInput, State> + NamedTuple + use<State>
where
    State: HasRand + HasMetadata + 'static,
{
    let swap = tuple_list![SwapRequests::new(SliceSwapMutator::new())];
    append_randomly_generated_messages(config)
        .merge(swap)
        .merge(message_reductions())
}

pub fn message_reductions<State>() -> tuple_list_type![DropRandomMessage<State>]
where
    State: HasRand,
{
    tuple_list![DropRandomMessage::new()]
}
