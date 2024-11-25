use std::marker::PhantomData;

use lsp_types::{
    CompletionContext, CompletionTriggerKind, PartialResultParams, SemanticTokensParams,
    TextDocumentIdentifier, TextDocumentPositionParams, WorkDoneProgressParams,
};

use crate::{
    lsp_input::{messages::PositionSelector, LspInput},
    text_document::mutations::TextDocumentSelector,
    utils::MapInner,
};

pub trait LspParamsGenerator<S> {
    type Result;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error>;
}

#[derive(Debug)]
pub struct FullSemanticTokens<D> {
    _doc: PhantomData<DocumentId<D>>,
}

impl<D, S> LspParamsGenerator<S> for FullSemanticTokens<D>
where
    D: TextDocumentSelector<S>,
{
    type Result = SemanticTokensParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        DocumentId::<D>::generate(state, input).map_inner(|text_document| SemanticTokensParams {
            text_document,
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
    }
}

#[derive(Debug)]
pub struct Hover<D, P> {
    _doc_pos: PhantomData<DocumentPosition<D, P>>,
}

impl<D, P, S> LspParamsGenerator<S> for Hover<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::HoverParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        DocumentPosition::<D, P>::generate(state, input).map_inner(
            |text_document_position_params| lsp_types::HoverParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document_position_params,
            },
        )
    }
}

#[derive(Debug)]
pub struct GoToDef<D, P> {
    _doc_pos: PhantomData<DocumentPosition<D, P>>,
}

impl<D, P, S> LspParamsGenerator<S> for GoToDef<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::GotoDefinitionParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        DocumentPosition::<D, P>::generate(state, input).map_inner(
            |text_document_position_params| lsp_types::GotoDefinitionParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document_position_params,
                partial_result_params: PartialResultParams::default(),
            },
        )
    }
}

#[derive(Debug)]
pub struct TriggerCompletion<D, P> {
    _doc_pos: PhantomData<DocumentPosition<D, P>>,
}

impl<D, P, S> LspParamsGenerator<S> for TriggerCompletion<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::CompletionParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        DocumentPosition::<D, P>::generate(state, input).map_inner(|text_document_position| {
            lsp_types::CompletionParams {
                text_document_position,
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
                work_done_progress_params: WorkDoneProgressParams::default(),
            }
        })
    }
}

#[derive(Debug)]
pub struct InlayHintWholdDoc<D> {
    _doc: PhantomData<D>,
}

impl<D, S> LspParamsGenerator<S> for InlayHintWholdDoc<D>
where
    D: TextDocumentSelector<S>,
{
    type Result = lsp_types::InlayHintParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((uri, doc)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let text_document = TextDocumentIdentifier { uri };
        let params = lsp_types::InlayHintParams {
            text_document,
            work_done_progress_params: WorkDoneProgressParams::default(),
            range: doc.lsp_range(),
        };
        Ok(Some(params))
    }
}
#[derive(Debug)]
pub struct TypeHierarchyPrep<D, P> {
    _doc_pos: PhantomData<DocumentPosition<D, P>>,
}

impl<D, P, S> LspParamsGenerator<S> for TypeHierarchyPrep<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::TypeHierarchyPrepareParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        DocumentPosition::<D, P>::generate(state, input).map_inner(
            |text_document_position_params| lsp_types::TypeHierarchyPrepareParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document_position_params,
            },
        )
    }
}

#[derive(Debug)]
pub struct DocumentId<D> {
    _document: PhantomData<D>,
}

impl<D, S> LspParamsGenerator<S> for DocumentId<D>
where
    D: TextDocumentSelector<S>,
{
    type Result = TextDocumentIdentifier;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((uri, _)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let text_document = TextDocumentIdentifier { uri };
        Ok(Some(text_document))
    }
}

#[derive(Debug)]
pub struct DocumentPosition<D, P> {
    _document: PhantomData<DocumentId<D>>,
    _position: PhantomData<P>,
}

impl<D, P, S> LspParamsGenerator<S> for DocumentPosition<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = TextDocumentPositionParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((uri, doc)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let text_document = TextDocumentIdentifier { uri };
        let Some(position) = P::select_position(state, doc) else {
            return Ok(None);
        };
        let params = TextDocumentPositionParams {
            text_document,
            position,
        };
        Ok(Some(params))
    }
}
#[derive(Debug)]
pub struct FindReferences<D, P> {
    _doc_pos: PhantomData<DocumentPosition<D, P>>,
}

impl<D, P, S> LspParamsGenerator<S> for FindReferences<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::ReferenceParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        DocumentPosition::<D, P>::generate(state, input).map_inner(|text_document_position| {
            lsp_types::ReferenceParams {
                text_document_position,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: lsp_types::ReferenceContext {
                    include_declaration: true,
                },
            }
        })
    }
}

#[derive(Debug)]
pub struct DocumentHighlight<D, P> {
    _doc_pos: PhantomData<DocumentPosition<D, P>>,
}

impl<D, P, S> LspParamsGenerator<S> for DocumentHighlight<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::DocumentHighlightParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        DocumentPosition::<D, P>::generate(state, input).map_inner(
            |text_document_position_params| lsp_types::DocumentHighlightParams {
                text_document_position_params,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        )
    }
}
