#[allow(
    clippy::wildcard_imports,
    reason = "LSP parameter types are dense here"
)]
use lsp_types::*;
use trait_gen::trait_gen;
use tuple_list::{TupleList, tuple_list_type};

use crate::lsp::generation::doc_range::{DocumentSelection, Selection};

compose! {
    #[trait_gen(T ->
        GotoDefinitionParams,
        DocumentHighlightParams,
        MonikerParams,
    )]
    T {
        text_document_position_params: TextDocumentPositionParams,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

compose! {
    #[trait_gen(T ->
        HoverParams,
        LinkedEditingRangeParams,
        CallHierarchyPrepareParams,
        TypeHierarchyPrepareParams
    )]
    T {
        text_document_position_params: TextDocumentPositionParams,
        work_done_progress_params: WorkDoneProgressParams
    }
}

impl crate::lsp::Compose for SignatureHelpContext {
    type Components = tuple_list_type![SignatureHelpTriggerKind, Option<String>, bool,];

    fn compose(components: Self::Components) -> Self {
        let (trigger_kind, trigger_character, is_retrigger) = components.into_tuple();
        Self {
            trigger_kind,
            trigger_character,
            is_retrigger,
            active_signature_help: None,
        }
    }
}

compose! {
    SignatureHelpParams {
        context: Option<SignatureHelpContext>,
        text_document_position_params: TextDocumentPositionParams,
        work_done_progress_params: WorkDoneProgressParams
    }
}

compose! {
    ReferenceParams {
        text_document_position: TextDocumentPositionParams,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams,
        context: ReferenceContext
    }
}

compose! {
    ReferenceContext {
        include_declaration: bool
    }
}

compose! {
    CompletionParams {
        text_document_position: TextDocumentPositionParams,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams,
        context: Option<CompletionContext>
    }
}

compose! {
    CompletionContext {
        trigger_kind: CompletionTriggerKind,
        trigger_character: Option<String>
    }
}

compose! {
    RenameParams {
        text_document_position: TextDocumentPositionParams,
        new_name: String,
        work_done_progress_params: WorkDoneProgressParams
    }
}

impl crate::lsp::Compose for SelectionRangeParams {
    type Components = tuple_list_type![
        TextDocumentPositionParams,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (
            TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params,
            partial_result_params,
        ) = components.into_tuple();
        Self {
            text_document,
            positions: vec![position],
            work_done_progress_params,
            partial_result_params,
        }
    }
}

#[trait_gen(T ->
    InlayHintParams,
)]
impl crate::lsp::Compose for T {
    type Components = tuple_list_type![Selection, WorkDoneProgressParams,];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (DocumentSelection(text_document, range), work_done_progress_params) =
            components.into_tuple();
        Self {
            text_document,
            work_done_progress_params,
            range,
        }
    }
}
