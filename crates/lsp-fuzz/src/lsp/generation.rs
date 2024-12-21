use std::{marker::PhantomData, rc::Rc};

use derive_new::new as New;
use itertools::Itertools;
use lsp_types::{
    CallHierarchyPrepareParams, CodeLensParams, DocumentColorParams, DocumentDiagnosticParams,
    DocumentHighlightParams, DocumentLinkParams, DocumentSymbolParams, GotoDefinitionParams,
    HoverParams, PartialResultParams, ReferenceContext, ReferenceParams, SemanticTokensParams,
    TextDocumentIdentifier, TextDocumentPositionParams, TypeHierarchyPrepareParams,
    WorkDoneProgressParams, WorkspaceSymbolParams,
};
use trait_gen::trait_gen;
use tuple_list::{tuple_list_type, TupleList};

use crate::{
    lsp_input::{
        messages::{HasPredefinedGenerators, PositionSelector},
        LspInput,
    },
    text_document::mutations::TextDocumentSelector,
};

pub trait LspParamsGenerator<S> {
    type Output;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError>;

    fn map<F, U>(self, f: F) -> MappingGenerator<S, Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> U,
    {
        MappingGenerator::new(self, f)
    }
}

impl<S, T> LspParamsGenerator<S> for Rc<dyn LspParamsGenerator<S, Output = T> + 'static> {
    type Output = T;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        self.as_ref().generate(state, input)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("Nothing was generated")]
    NothingGenerated,
    #[error(transparent)]
    Error(#[from] libafl::Error),
}

#[derive(Debug, Clone, New)]
pub struct ConstGenerator<T> {
    value: T,
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

#[derive(Debug, New)]
pub struct DefaultGenerator<S, T> {
    _phantom: PhantomData<(S, T)>,
}

impl<S, T> LspParamsGenerator<S> for DefaultGenerator<S, T>
where
    T: Default,
{
    type Output = T;

    fn generate(&self, _state: &mut S, _input: &LspInput) -> Result<Self::Output, GenerationError> {
        Ok(T::default())
    }
}

#[derive(Debug, New)]
pub struct TextDocumentIdentifierGenerator<S, D> {
    _phantom: PhantomData<(S, D)>,
}

type Type = lsp_types::TextDocumentIdentifier;

impl<S, D> LspParamsGenerator<S> for TextDocumentIdentifierGenerator<S, D>
where
    D: TextDocumentSelector<S>,
{
    type Output = Type;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        let (uri, _) = D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output { uri })
    }
}

#[derive(Debug, New)]
pub struct TextDocumentPositionParamsGenerator<S, D, P> {
    _phantom: PhantomData<(S, D, P)>,
}

impl<S, D, P> LspParamsGenerator<S> for TextDocumentPositionParamsGenerator<S, D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Output = TextDocumentPositionParams;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let position = P::select_position(state, doc).ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output {
            text_document: lsp_types::TextDocumentIdentifier { uri },
            position,
        })
    }
}

#[derive(Debug, New)]
pub struct MappingGenerator<S, G, F> {
    generator: G,
    f: F,
    _phantom: PhantomData<S>,
}

impl<S, G, F, T, U> LspParamsGenerator<S> for MappingGenerator<S, G, F>
where
    G: LspParamsGenerator<S, Output = T>,
    F: Fn(T) -> U,
{
    type Output = U;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        let value = self.generator.generate(state, input)?;
        Ok((self.f)(value))
    }
}

impl<S, T, T1, T2> HasPredefinedGenerators<S> for T
where
    T1: HasPredefinedGenerators<S> + 'static,
    T2: HasPredefinedGenerators<S> + 'static,
    T: CompositeOf<Components = (T1, T2)> + 'static,
{
    fn generators() -> Vec<Rc<dyn LspParamsGenerator<S, Output = Self>>>
    where
        S: 'static,
    {
        let t1_generators = T1::generators();
        let t2_generators = T2::generators();
        t1_generators
            .into_iter()
            .cartesian_product(t2_generators)
            .map(|(g1, g2)| Rc::new(CompositeGenerator::<S, _, _, T>::new(g1, g2)) as _)
            .collect()
    }
}

#[derive(Debug, New)]
pub struct CompositeGenerator<S, G1, G2, T> {
    generator1: G1,
    generator2: G2,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, G1, G2> LspParamsGenerator<S> for CompositeGenerator<S, G1, G2, T>
where
    G1: LspParamsGenerator<S>,
    G2: LspParamsGenerator<S>,
    T: CompositeOf<Components = (G1::Output, G2::Output)> + 'static,
{
    type Output = T;

    fn generate(&self, state: &mut S, input: &LspInput) -> Result<Self::Output, GenerationError> {
        let c1 = self.generator1.generate(state, input)?;
        let c2 = self.generator2.generate(state, input)?;
        let output = T::compose((c1, c2));
        Ok(output)
    }
}

pub trait CompositeOf {
    type Components;

    fn compose(components: Self::Components) -> Self;
}

impl<Head, Tail> CompositeOf for (Head, Tail) {
    type Components = (Head, Tail);

    fn compose(components: Self::Components) -> Self {
        components
    }
}

#[trait_gen(T ->
    GotoDefinitionParams,
    DocumentHighlightParams,
)]
impl CompositeOf for T {
    type Components = tuple_list_type![
        TextDocumentPositionParams,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (text_document_position_params, work_done_progress_params, partial_result_params) =
            components.into_tuple();
        Self {
            text_document_position_params,
            work_done_progress_params,
            partial_result_params,
        }
    }
}

impl CompositeOf for ReferenceParams {
    type Components = tuple_list_type![
        TextDocumentPositionParams,
        WorkDoneProgressParams,
        PartialResultParams,
        ReferenceContext
    ];

    fn compose(components: Self::Components) -> Self {
        let (text_document_position, work_done_progress_params, partial_result_params, context) =
            components.into_tuple();
        Self {
            text_document_position,
            work_done_progress_params,
            partial_result_params,
            context,
        }
    }
}

impl CompositeOf for ReferenceContext {
    type Components = tuple_list_type![bool];

    fn compose(components: Self::Components) -> Self {
        let (include_declaration,) = components.into_tuple();
        Self {
            include_declaration,
        }
    }
}

#[trait_gen(T ->
    CallHierarchyPrepareParams,
    TypeHierarchyPrepareParams,
    HoverParams
)]
impl CompositeOf for T {
    type Components = tuple_list_type![TextDocumentPositionParams, WorkDoneProgressParams];

    fn compose(components: Self::Components) -> Self {
        let (text_document_position_params, work_done_progress_params) = components.into_tuple();
        Self {
            text_document_position_params,
            work_done_progress_params,
        }
    }
}

impl CompositeOf for DocumentDiagnosticParams {
    type Components = tuple_list_type![
        TextDocumentIdentifier,
        Option<String>,
        Option<String>,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (
            text_document,
            identifier,
            previous_result_id,
            work_done_progress_params,
            partial_result_params,
        ) = components.into_tuple();
        Self {
            text_document,
            identifier,
            previous_result_id,
            work_done_progress_params,
            partial_result_params,
        }
    }
}

impl CompositeOf for WorkspaceSymbolParams {
    type Components = tuple_list_type![String, WorkDoneProgressParams, PartialResultParams];

    fn compose(components: Self::Components) -> Self {
        let (query, work_done_progress_params, partial_result_params) = components.into_tuple();
        Self {
            query,
            work_done_progress_params,
            partial_result_params,
        }
    }
}

#[trait_gen(T ->
    SemanticTokensParams,
    DocumentSymbolParams,
    DocumentLinkParams,
    DocumentColorParams,
    CodeLensParams,
)]
impl CompositeOf for T {
    type Components = tuple_list_type![
        TextDocumentIdentifier,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (text_document, work_done_progress_params, partial_result_params) =
            components.into_tuple();
        Self {
            work_done_progress_params,
            partial_result_params,
            text_document,
        }
    }
}
