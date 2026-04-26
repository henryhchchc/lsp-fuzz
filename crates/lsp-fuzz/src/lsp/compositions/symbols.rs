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
        SemanticTokensParams,
        DocumentSymbolParams,
        DocumentLinkParams,
        DocumentColorParams,
        CodeLensParams,
    )]
    T {
        text_document: TextDocumentIdentifier,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

impl crate::lsp::Compose for SemanticTokensRangeParams {
    type Components = tuple_list_type![Selection, WorkDoneProgressParams, PartialResultParams];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (
            DocumentSelection(text_document, range),
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

compose! {
    SemanticTokensDeltaParams {
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams,
        text_document: TextDocumentIdentifier,
        previous_result_id: String
    }
}
