use std::{
    marker::{PhantomData, Sized},
    num::NonZeroUsize,
    ops::Deref,
};

use derive_new::new as New;
use libafl::{mutators::Tokens, state::HasRand, HasMetadata};
use libafl_bolts::rands::Rand;
use lsp_types::{
    CodeActionKind, CodeActionTriggerKind, CompletionTriggerKind, Position, Range, SetTraceParams,
    SignatureHelpTriggerKind, TextDocumentIdentifier, TraceValue,
};

use crate::{
    lsp_input::{messages::PositionSelector, LspInput},
    macros::const_generators,
    text_document::{
        mutations::{text_document_selectors::RandomDoc, TextDocumentSelector},
        TextDocument,
    },
};

use super::{Compose, HasPredefinedGenerators};

pub trait LspParamsGenerator<S> {
    type Output;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError>;
}

impl<S, G, Ptr> LspParamsGenerator<S> for Ptr
where
    Ptr: Deref<Target = G>,
    G: LspParamsGenerator<S> + ?Sized,
{
    type Output = G::Output;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
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

impl<S, T> LspParamsGenerator<S> for ConstGenerator<T>
where
    T: Clone,
{
    type Output = T;

    fn generate(&self, _state: &mut S, _input: &LspInput) -> Result<Self::Output, GenerationError> {
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

impl<S, T> LspParamsGenerator<S> for DefaultGenerator<T>
where
    T: Default,
{
    type Output = T;

    fn generate(&self, _state: &mut S, _input: &LspInput) -> Result<Self::Output, GenerationError> {
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

impl<S, D> LspParamsGenerator<S> for TextDocumentIdentifierGenerator<D>
where
    D: TextDocumentSelector<S>,
{
    type Output = TextDocumentIdentifier;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        let (uri, _) = D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output { uri })
    }
}

#[derive(Debug, New)]
pub struct TextDocumentPositionParamsGenerator<D, P> {
    _phantom: PhantomData<(D, P)>,
}

impl<S, D, P> LspParamsGenerator<S> for TextDocumentPositionParamsGenerator<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Output = lsp_types::TextDocumentPositionParams;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
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
pub struct MappingGenerator<S, G, T, U> {
    generator: G,
    mapper: fn(T) -> U,
    _phantom: PhantomData<S>,
}

impl<S, G, T, U> MappingGenerator<S, G, T, U> {
    pub const fn new(generator: G, mapper: fn(T) -> U) -> Self {
        Self {
            generator,
            mapper,
            _phantom: PhantomData,
        }
    }
}

impl<S, G, T, U> Clone for MappingGenerator<S, G, T, U>
where
    G: Clone,
{
    fn clone(&self) -> Self {
        let generator = self.generator.clone();
        Self::new(generator, self.mapper)
    }
}

impl<S, G, T, U> LspParamsGenerator<S> for MappingGenerator<S, G, T, U>
where
    G: LspParamsGenerator<S, Output = T>,
{
    type Output = U;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        self.generator
            .generate(state, input)
            .map(|it| (self.mapper)(it))
    }
}

impl<S, T, T1, T2> HasPredefinedGenerators<S> for T
where
    T1: HasPredefinedGenerators<S> + 'static,
    T2: HasPredefinedGenerators<S> + 'static,
    T: Compose<Components = (T1, T2)> + 'static,
    T1::Generator: Clone,
    T2::Generator: Clone,
{
    type Generator = CompositionGenerator<T1::Generator, T2::Generator, Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        S: 'static,
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

impl<S, T, G1, G2> LspParamsGenerator<S> for CompositionGenerator<G1, G2, T>
where
    G1: LspParamsGenerator<S>,
    G2: LspParamsGenerator<S>,
    T: Compose<Components = (G1::Output, G2::Output)>,
{
    type Output = T;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        let c1 = self.generator1.generate(state, input)?;
        let c2 = self.generator2.generate(state, input)?;
        let output = T::compose((c1, c2));
        Ok(output)
    }
}

#[derive(Debug, Default)]
pub struct TokensGenerator<T> {
    _phantom: PhantomData<T>,
}

impl<T> TokensGenerator<T> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S> LspParamsGenerator<S> for TokensGenerator<String>
where
    S: HasMetadata + HasRand,
{
    type Output = String;

    fn generate(&self, state: &mut S, _input: &LspInput) -> Result<Self::Output, GenerationError> {
        let token_cnt = {
            let tokens: &Tokens = state
                .metadata()
                .map_err(|_| GenerationError::NothingGenerated)?;
            NonZeroUsize::new(tokens.len()).ok_or(GenerationError::NothingGenerated)?
        };
        let idx = state.rand_mut().below(token_cnt);
        // SAFETY: We checked just now that the metadata is present
        let tokens: &Tokens = unsafe { state.metadata().unwrap_unchecked() };
        let token = String::from_utf8_lossy(&tokens[idx]).into_owned();
        Ok(token)
    }
}

#[derive(Debug)]
pub struct RangeInDoc(pub TextDocumentIdentifier, pub Range);

#[derive(Debug, New)]
pub struct RangeInDocGenerator<S, D> {
    range_selector: fn(&mut S, &TextDocument) -> Range,
    _phantom: PhantomData<(S, D)>,
}

impl<S, D> Clone for RangeInDocGenerator<S, D> {
    fn clone(&self) -> Self {
        Self::new(self.range_selector)
    }
}

impl<S, D> LspParamsGenerator<S> for RangeInDocGenerator<S, D>
where
    D: TextDocumentSelector<S>,
{
    type Output = RangeInDoc;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let range = (self.range_selector)(state, doc);
        let doc = TextDocumentIdentifier { uri };
        Ok(RangeInDoc(doc, range))
    }
}

impl<S> HasPredefinedGenerators<S> for RangeInDoc
where
    S: HasRand,
{
    type Generator = RangeInDocGenerator<S, RandomDoc<S>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        S: 'static,
    {
        let whole_range = |_: &mut S, doc: &TextDocument| doc.lsp_range();
        let after_range = |_: &mut S, doc: &TextDocument| {
            let Range { end, .. } = doc.lsp_range();
            let start = end;
            let end = Position {
                line: 65536,
                character: end.character,
            };
            Range { start, end }
        };
        let inverted_range = |_: &mut S, doc: &TextDocument| {
            let Range { start, end } = doc.lsp_range();
            Range {
                start: end,
                end: start,
            }
        };
        [whole_range, after_range, inverted_range].map(RangeInDocGenerator::new)
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
