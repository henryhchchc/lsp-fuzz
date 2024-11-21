use std::marker::PhantomData;

use lsp_types::{
    CompletionContext, CompletionTriggerKind, PartialResultParams, SemanticTokensParams,
    TextDocumentIdentifier, TextDocumentPositionParams, WorkDoneProgressParams,
};

use crate::{
    lsp_input::{messages::PositionSelector, LspInput},
    text_document::mutations::TextDocumentSelector,
};

pub trait LspParamsGenerator<S> {
    type Result;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error>;
}

#[derive(Debug)]
pub struct FullSemanticTokens<D>(PhantomData<D>);

impl<D, S> LspParamsGenerator<S> for FullSemanticTokens<D>
where
    D: TextDocumentSelector<S>,
{
    type Result = SemanticTokensParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((path, _)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let uri = format!("lsp-fuzz://{}", path.display()).parse().unwrap();
        let text_document = TextDocumentIdentifier { uri };
        let params = SemanticTokensParams {
            text_document,
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        Ok(Some(params))
    }
}

#[derive(Debug)]
pub struct Hover<D, P> {
    _document: PhantomData<D>,
    _position: PhantomData<P>,
}

impl<D, P, S> LspParamsGenerator<S> for Hover<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::HoverParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((path, doc)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let uri = format!("lsp-fuzz://{}", path.display()).parse().unwrap();
        let text_document = TextDocumentIdentifier { uri };
        let Some(position) = P::select_position(state, doc) else {
            return Ok(None);
        };
        let params = lsp_types::HoverParams {
            work_done_progress_params: WorkDoneProgressParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document,
                position,
            },
        };
        Ok(Some(params))
    }
}

#[derive(Debug)]
pub struct GoToDef<D, P> {
    _document: PhantomData<D>,
    _position: PhantomData<P>,
}

impl<D, P, S> LspParamsGenerator<S> for GoToDef<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::GotoDefinitionParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((path, doc)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let uri = format!("lsp-fuzz://{}", path.display()).parse().unwrap();
        let text_document = TextDocumentIdentifier { uri };
        let Some(position) = P::select_position(state, doc) else {
            return Ok(None);
        };
        let params = lsp_types::GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        Ok(Some(params))
    }
}

#[derive(Debug)]
pub struct TriggerCompletion<D, P> {
    _document: PhantomData<D>,
    _position: PhantomData<P>,
}

impl<D, P, S> LspParamsGenerator<S> for TriggerCompletion<D, P>
where
    D: TextDocumentSelector<S>,
    P: PositionSelector<S>,
{
    type Result = lsp_types::CompletionParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((path, doc)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let uri = format!("lsp-fuzz://{}", path.display()).parse().unwrap();
        let text_document = TextDocumentIdentifier { uri };
        let Some(position) = P::select_position(state, doc) else {
            return Ok(None);
        };
        let params = lsp_types::CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document,
                position,
            },
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        Ok(Some(params))
    }
}

#[derive(Debug)]
pub struct RequestInlayHint<D>(PhantomData<D>);

impl<D, S> LspParamsGenerator<S> for RequestInlayHint<D>
where
    D: TextDocumentSelector<S>,
{
    type Result = lsp_types::InlayHintParams;

    fn generate(state: &mut S, input: &LspInput) -> Result<Option<Self::Result>, libafl::Error> {
        let Some((path, doc)) = D::select_document(state, input) else {
            return Ok(None);
        };
        let uri = format!("lsp-fuzz://{}", path.display()).parse().unwrap();
        let text_document = TextDocumentIdentifier { uri };
        let params = lsp_types::InlayHintParams {
            text_document,
            work_done_progress_params: WorkDoneProgressParams::default(),
            range: doc.lsp_range(),
        };
        Ok(Some(params))
    }
}
