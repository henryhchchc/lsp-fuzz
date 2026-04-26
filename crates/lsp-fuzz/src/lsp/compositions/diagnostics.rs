#[allow(
    clippy::wildcard_imports,
    reason = "LSP parameter types are dense here"
)]
use lsp_types::*;
use tuple_list::{TupleList, tuple_list_type};

use crate::lsp::generation::doc_range::{DocumentSelection, Selection};

compose! {
    DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier,
        identifier: Option<String>,
        previous_result_id: Option<String>,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

compose! {
    WorkspaceDiagnosticParams {
        identifier: Option<String>,
        previous_result_ids: Vec<PreviousResultId>,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

impl crate::lsp::Compose for CodeActionParams {
    type Components = tuple_list_type![
        Selection,
        WorkDoneProgressParams,
        PartialResultParams,
        CodeActionContext
    ];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (
            DocumentSelection(text_document, range),
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

impl crate::lsp::Compose for CodeActionContext {
    type Components = tuple_list_type![Option<Vec<CodeActionKind>>, Option<CodeActionTriggerKind>];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (only, trigger_kind) = components.into_tuple();
        Self {
            diagnostics: vec![],
            only,
            trigger_kind,
        }
    }
}
