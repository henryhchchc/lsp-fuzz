use std::{
    marker::{PhantomData, Sized},
    ops::Deref,
    rc::Rc,
};

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
pub struct MappingGenerator<S, G, T, U> {
    generator: G,
    mapper: fn(T) -> U,
    _phantom: PhantomData<S>,
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
            .map(|(g1, g2)| Rc::new(CompositionGenerator::new(g1, g2)) as _)
            .collect()
    }
}

#[derive(Debug, New)]
pub struct CompositionGenerator<S, G1, G2, T> {
    generator1: G1,
    generator2: G2,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, G1, G2> LspParamsGenerator<S> for CompositionGenerator<S, G1, G2, T>
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

pub trait Compose {
    type Components;

    fn compose(components: Self::Components) -> Self;
}

impl<Head, Tail> Compose for (Head, Tail) {
    type Components = (Head, Tail);

    #[inline]
    fn compose(components: Self::Components) -> Self {
        components
    }
}

#[trait_gen(T ->
    GotoDefinitionParams,
    DocumentHighlightParams,
)]
impl Compose for T {
    type Components = tuple_list_type![
        TextDocumentPositionParams,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    #[inline]
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

impl Compose for ReferenceParams {
    type Components = tuple_list_type![
        TextDocumentPositionParams,
        WorkDoneProgressParams,
        PartialResultParams,
        ReferenceContext
    ];

    #[inline]
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

impl Compose for ReferenceContext {
    type Components = tuple_list_type![bool];

    #[inline]
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
impl Compose for T {
    type Components = tuple_list_type![TextDocumentPositionParams, WorkDoneProgressParams];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (text_document_position_params, work_done_progress_params) = components.into_tuple();
        Self {
            text_document_position_params,
            work_done_progress_params,
        }
    }
}

impl Compose for DocumentDiagnosticParams {
    type Components = tuple_list_type![
        TextDocumentIdentifier,
        Option<String>,
        Option<String>,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    #[inline]
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

impl Compose for WorkspaceSymbolParams {
    type Components = tuple_list_type![String, WorkDoneProgressParams, PartialResultParams];

    #[inline]
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
impl Compose for T {
    type Components = tuple_list_type![
        TextDocumentIdentifier,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    #[inline]
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
