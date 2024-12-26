use lsp_types::{
    CallHierarchyPrepareParams, CodeActionContext, CodeActionParams, CodeLensParams,
    CompletionContext, CompletionParams, CompletionTriggerKind, DocumentColorParams,
    DocumentDiagnosticParams, DocumentHighlightParams, DocumentLinkParams, DocumentSymbolParams,
    GotoDefinitionParams, HoverParams, InlayHintParams, LinkedEditingRangeParams,
    PartialResultParams, ReferenceContext, ReferenceParams, SemanticTokensParams,
    SemanticTokensRangeParams, TextDocumentIdentifier, TextDocumentPositionParams,
    TypeHierarchyPrepareParams, WorkDoneProgressParams, WorkspaceSymbolParams,
};
use trait_gen::trait_gen;
use tuple_list::{tuple_list_type, TupleList};

use super::{generation::DocAndRange, Compose};

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
    HoverParams,
    LinkedEditingRangeParams
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

#[trait_gen(T ->
    InlayHintParams,
)]

impl Compose for T {
    type Components = tuple_list_type![DocAndRange, WorkDoneProgressParams,];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (
            DocAndRange {
                text_document,
                range,
            },
            work_done_progress_params,
        ) = components.into_tuple();
        Self {
            text_document,
            work_done_progress_params,
            range,
        }
    }
}

impl Compose for CompletionParams {
    type Components = tuple_list_type![
        TextDocumentPositionParams,
        WorkDoneProgressParams,
        PartialResultParams,
        Option<CompletionContext>
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

impl Compose for CompletionContext {
    type Components = tuple_list_type![CompletionTriggerKind, Option<String>];

    fn compose(components: Self::Components) -> Self {
        let (trigger_kind, trigger_character) = components.into_tuple();
        Self {
            trigger_kind,
            trigger_character,
        }
    }
}

impl Compose for SemanticTokensRangeParams {
    type Components = tuple_list_type![DocAndRange, WorkDoneProgressParams, PartialResultParams];

    fn compose(components: Self::Components) -> Self {
        let (
            DocAndRange {
                text_document,
                range,
            },
            work_done_progress_params,
            partial_result_params,
        ) = components.into_tuple();
        Self {
            work_done_progress_params,
            partial_result_params,
            text_document,
            range,
        }
    }
}

impl Compose for CodeActionParams {
    type Components = tuple_list_type![
        DocAndRange,
        WorkDoneProgressParams,
        PartialResultParams,
        CodeActionContext
    ];

    fn compose(components: Self::Components) -> Self {
        let (
            DocAndRange {
                text_document,
                range,
            },
            work_done_progress_params,
            partial_result_params,
            context,
        ) = components.into_tuple();
        Self {
            text_document,
            range,
            work_done_progress_params,
            partial_result_params,
            context,
        }
    }
}

impl Compose for CodeActionContext {
    type Components = tuple_list_type![()];

    fn compose(_components: Self::Components) -> Self {
        // TODO: Implement this
        Self {
            diagnostics: vec![],
            only: None,
            trigger_kind: None,
        }
    }
}
