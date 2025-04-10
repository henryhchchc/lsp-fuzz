use std::{
    marker::{PhantomData, Sized},
    num::NonZeroUsize,
    ops::Deref,
    result::Result,
};

use derive_new::new as New;
use libafl::{HasMetadata, state::HasRand};
use libafl_bolts::rands::Rand;
use lsp_types::{
    CodeActionKind, CodeActionTriggerKind, CompletionTriggerKind, Position, Range, SetTraceParams,
    SignatureHelpTriggerKind, TextDocumentIdentifier, TraceValue,
};

use crate::{
    lsp_input::{LspInput, messages::PositionSelector},
    macros::const_generators,
    text_document::{
        GrammarBasedMutation, TextDocument,
        grammar::tree_sitter::TreeIter,
        mutations::{TextDocumentSelector, text_document_selectors::RandomDoc},
    },
    utf8::UTF8Tokens,
};

use super::{Compose, HasPredefinedGenerators};

pub trait LspParamsGenerator<State> {
    type Output;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError>;
}

impl<State, G, Ptr> LspParamsGenerator<State> for Ptr
where
    Ptr: Deref<Target = G>,
    G: LspParamsGenerator<State> + ?Sized,
{
    type Output = G::Output;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        self.deref().generate(state, input)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("Nothing was generated")]
    NothingGenerated,
    #[error(transparent)]
    Error(#[from] libafl::Error),
}

#[derive(Debug, Clone)]
pub struct ConstGenerator<T> {
    value: T,
}

impl<T> ConstGenerator<T> {
    pub const fn new(value: T) -> Self {
        Self { value }
    }
}

impl<State, T> LspParamsGenerator<State> for ConstGenerator<T>
where
    T: Clone,
{
    type Output = T;

    fn generate(
        &self,
        _state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        Ok(self.value.clone())
    }
}

#[derive(Debug)]
pub struct DefaultGenerator<T> {
    _phantom: PhantomData<T>,
}

impl<T> DefaultGenerator<T> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for DefaultGenerator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for DefaultGenerator<T> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<State, T> LspParamsGenerator<State> for DefaultGenerator<T>
where
    T: Default,
{
    type Output = T;

    fn generate(
        &self,
        _state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        Ok(T::default())
    }
}

#[derive(Debug, New)]
pub struct TextDocumentIdentifierGenerator<D> {
    _phantom: PhantomData<D>,
}

impl<T> Clone for TextDocumentIdentifierGenerator<T> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<State, D> LspParamsGenerator<State> for TextDocumentIdentifierGenerator<D>
where
    D: TextDocumentSelector<State>,
{
    type Output = TextDocumentIdentifier;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, _) = D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output { uri })
    }
}

#[derive(Debug, New)]
pub struct TextDocumentPositionParamsGenerator<D, P> {
    _phantom: PhantomData<(D, P)>,
}

impl<State, D, P> LspParamsGenerator<State> for TextDocumentPositionParamsGenerator<D, P>
where
    D: TextDocumentSelector<State>,
    P: PositionSelector<State>,
{
    type Output = lsp_types::TextDocumentPositionParams;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let position = P::select_position(state, doc).ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output {
            text_document: TextDocumentIdentifier { uri },
            position,
        })
    }
}

#[derive(Debug)]
pub struct MappingGenerator<State, G, T, U> {
    generator: G,
    mapper: fn(T) -> U,
    _phantom: PhantomData<State>,
}

impl<State, G, T, U> MappingGenerator<State, G, T, U> {
    pub const fn new(generator: G, mapper: fn(T) -> U) -> Self {
        Self {
            generator,
            mapper,
            _phantom: PhantomData,
        }
    }
}

impl<State, G, T, U> Clone for MappingGenerator<State, G, T, U>
where
    G: Clone,
{
    fn clone(&self) -> Self {
        let generator = self.generator.clone();
        Self::new(generator, self.mapper)
    }
}

impl<State, G, T, U> LspParamsGenerator<State> for MappingGenerator<State, G, T, U>
where
    G: LspParamsGenerator<State, Output = T>,
{
    type Output = U;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        self.generator.generate(state, input).map(self.mapper)
    }
}

impl<State, T, T1, T2> HasPredefinedGenerators<State> for T
where
    T1: HasPredefinedGenerators<State> + 'static,
    T2: HasPredefinedGenerators<State> + 'static,
    T: Compose<Components = (T1, T2)> + 'static,
    T1::Generator: Clone,
    T2::Generator: Clone,
{
    type Generator = CompositionGenerator<T1::Generator, T2::Generator, Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: 'static,
    {
        let t1_generators = T1::generators();
        t1_generators.into_iter().flat_map(|g1| {
            T2::generators()
                .into_iter()
                .map(move |g2| CompositionGenerator::new(g1.clone(), g2.clone()))
        })
    }
}

#[derive(Debug)]
pub struct CompositionGenerator<G1, G2, T> {
    generator1: G1,
    generator2: G2,
    _phantom: PhantomData<fn() -> T>,
}

impl<G1, G2, T> CompositionGenerator<G1, G2, T> {
    pub const fn new(generator1: G1, generator2: G2) -> Self {
        Self {
            generator1,
            generator2,
            _phantom: PhantomData,
        }
    }
}

impl<G1, G2, T> Clone for CompositionGenerator<G1, G2, T>
where
    G1: Clone,
    G2: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.generator1.clone(), self.generator2.clone())
    }
}

impl<State, T, G1, G2> LspParamsGenerator<State> for CompositionGenerator<G1, G2, T>
where
    G1: LspParamsGenerator<State>,
    G2: LspParamsGenerator<State>,
    T: Compose<Components = (G1::Output, G2::Output)>,
{
    type Output = T;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let c1 = self.generator1.generate(state, input)?;
        let c2 = self.generator2.generate(state, input)?;
        let output = T::compose((c1, c2));
        Ok(output)
    }
}

#[derive(Debug, Default)]
pub struct UTF8TokensGenerator<T> {
    _phantom: PhantomData<T>,
}

impl<T> UTF8TokensGenerator<T> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<State> LspParamsGenerator<State> for UTF8TokensGenerator<String>
where
    State: HasMetadata + HasRand,
{
    type Output = String;

    fn generate(
        &self,
        state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let token_cnt = state
            .metadata()
            .map(UTF8Tokens::len)
            .ok()
            .and_then(NonZeroUsize::new)
            .ok_or(GenerationError::NothingGenerated)?;
        let idx = state.rand_mut().below(token_cnt);
        // SAFETY: We checked just now that the metadata is present
        let tokens: &UTF8Tokens = unsafe { state.metadata().unwrap_unchecked() };
        let token = tokens[idx].clone();
        Ok(token)
    }
}

#[derive(Debug)]
pub struct RangeInDoc(pub TextDocumentIdentifier, pub Range);

#[derive(Debug, New)]
pub struct RangeInDocGenerator<State, D> {
    range_selector: fn(&mut State, &TextDocument) -> Range,
    _phantom: PhantomData<D>,
}

impl<State, D> Clone for RangeInDocGenerator<State, D> {
    fn clone(&self) -> Self {
        Self::new(self.range_selector)
    }
}

impl<State, D> LspParamsGenerator<State> for RangeInDocGenerator<State, D>
where
    D: TextDocumentSelector<State>,
{
    type Output = RangeInDoc;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let range = (self.range_selector)(state, doc);
        let doc = TextDocumentIdentifier { uri };
        Ok(RangeInDoc(doc, range))
    }
}

impl<State> HasPredefinedGenerators<State> for RangeInDoc
where
    State: HasRand,
{
    type Generator = RangeInDocGenerator<State, RandomDoc<State>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: HasRand + 'static,
    {
        fn lsp_whole_range(doc: &TextDocument) -> Range {
            let start = lsp_types::Position::default();
            let end = doc
                .lines()
                .enumerate()
                .last()
                .map(|(line_idx, line)| lsp_types::Position::new(line_idx as _, line.len() as _))
                .unwrap_or_default();
            lsp_types::Range::new(start, end)
        }
        let whole_range = |_: &mut State, doc: &TextDocument| lsp_whole_range(doc);
        let random_range = |state: &mut State, doc: &TextDocument| {
            let rand = state.rand_mut();
            let lines: Vec<_> = doc.lines().collect();
            let start_line_idx = rand.below_or_zero(lines.len());
            let start_line = lines[start_line_idx];
            let end_line_idx = rand.between(start_line_idx, lines.len() - 1);
            let end_line = lines[end_line_idx];
            let start = Position {
                line: start_line_idx as u32,
                character: rand.below_or_zero(start_line.len()) as u32,
            };
            let end = Position {
                line: end_line_idx as u32,
                character: rand.below_or_zero(end_line.len()) as u32,
            };
            Range { start, end }
        };
        let random_subtree = |state: &mut State, doc: &TextDocument| {
            let tree_iter = doc.parse_tree().iter();
            if let Some(node) = state.rand_mut().choose(tree_iter) {
                let start = node.start_position();
                let start = Position {
                    line: start.row as u32,
                    character: start.column as u32,
                };
                let end = node.end_position();
                let end = Position {
                    line: end.row as u32,
                    character: end.column as u32,
                };
                Range { start, end }
            } else {
                lsp_whole_range(doc)
            }
        };
        let after_range = |_: &mut State, doc: &TextDocument| {
            let Range { end, .. } = lsp_whole_range(doc);
            let start = end;
            let end = Position {
                line: 65536,
                character: end.character,
            };
            Range { start, end }
        };
        let inverted_range = |_: &mut State, doc: &TextDocument| {
            let Range { start, end } = lsp_whole_range(doc);
            Range {
                start: end,
                end: start,
            }
        };
        [
            whole_range,
            random_range,
            random_range,
            after_range,
            inverted_range,
            random_subtree,
        ]
        .map(RangeInDocGenerator::new)
    }
}

#[derive(Debug)]
pub struct ZeroToOne32(pub f32);

#[derive(Debug, Clone)]
pub struct ZeroToOne32Gen;

impl<State> LspParamsGenerator<State> for ZeroToOne32Gen
where
    State: HasRand,
{
    type Output = ZeroToOne32;

    fn generate(
        &self,
        state: &mut State,
        _input: &LspInput,
    ) -> Result<ZeroToOne32, GenerationError> {
        Ok(ZeroToOne32(state.rand_mut().next_float() as f32))
    }
}

impl<State> HasPredefinedGenerators<State> for ZeroToOne32
where
    State: HasRand,
{
    type Generator = ZeroToOne32Gen;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: 'static,
    {
        [ZeroToOne32Gen]
    }
}

#[derive(Debug)]
pub struct TabSize(pub u32);

#[derive(Debug, Clone)]
pub struct TabSizeGen;

impl<State> LspParamsGenerator<State> for TabSizeGen
where
    State: HasRand,
{
    type Output = TabSize;

    fn generate(&self, state: &mut State, _input: &LspInput) -> Result<TabSize, GenerationError> {
        let inner = match state.rand_mut().next() % 6 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 4,
            4 => 8,
            5 => state.rand_mut().next() as u32,
            _ => unreachable!("Modulo of 6 should not be greater than 5"),
        };
        Ok(TabSize(inner))
    }
}

impl<State> HasPredefinedGenerators<State> for TabSize
where
    State: HasRand,
{
    type Generator = TabSizeGen;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: 'static,
    {
        [TabSizeGen]
    }
}

const_generators!(for CompletionTriggerKind => [
    CompletionTriggerKind::INVOKED,
    CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS,
    CompletionTriggerKind::TRIGGER_CHARACTER
]);

const_generators!(for CodeActionTriggerKind => [
    CodeActionTriggerKind::INVOKED,
    CodeActionTriggerKind::AUTOMATIC
]);

const_generators!(for CodeActionKind => [
    CodeActionKind::EMPTY,
    CodeActionKind::QUICKFIX,
    CodeActionKind::REFACTOR,
    CodeActionKind::REFACTOR_EXTRACT,
    CodeActionKind::REFACTOR_INLINE,
    CodeActionKind::REFACTOR_REWRITE,
    CodeActionKind::SOURCE,
    CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
    CodeActionKind::SOURCE_FIX_ALL
]);

const_generators!(for SignatureHelpTriggerKind => [
    SignatureHelpTriggerKind::INVOKED,
    SignatureHelpTriggerKind::TRIGGER_CHARACTER,
    SignatureHelpTriggerKind::CONTENT_CHANGE
]);

const_generators!(for SetTraceParams => [
    SetTraceParams { value: TraceValue::Messages },
    SetTraceParams { value: TraceValue::Off },
    SetTraceParams { value: TraceValue::Verbose }
]);
