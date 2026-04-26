#[allow(
    clippy::wildcard_imports,
    reason = "LSP parameter types are dense here"
)]
use lsp_types::*;
use ordermap::OrderMap;
use tuple_list::{TupleList, tuple_list_type};

use crate::lsp::generation::{
    doc_range::{DocumentSelection, Selection},
    numeric::{TabSize, ZeroToOne32},
};

compose! {
    FoldingRangeParams {
        text_document: TextDocumentIdentifier,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

compose! {
    DocumentFormattingParams {
        text_document: TextDocumentIdentifier,
        options: FormattingOptions,
        work_done_progress_params: WorkDoneProgressParams
    }
}

impl crate::lsp::Compose for ColorPresentationParams {
    type Components = tuple_list_type![
        Selection,
        Color,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (
            DocumentSelection(text_document, range),
            color,
            work_done_progress_params,
            partial_result_params,
        ) = components.into_tuple();
        Self {
            text_document,
            color,
            range,
            work_done_progress_params,
            partial_result_params,
        }
    }
}

impl crate::lsp::Compose for Color {
    type Components = tuple_list_type![ZeroToOne32, ZeroToOne32, ZeroToOne32, ZeroToOne32];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (ZeroToOne32(red), ZeroToOne32(green), ZeroToOne32(blue), ZeroToOne32(alpha)) =
            components.into_tuple();
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl crate::lsp::Compose for FormattingOptions {
    type Components = tuple_list_type![TabSize, bool, Option<bool>, Option<bool>, Option<bool>];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (
            TabSize(tab_size),
            insert_spaces,
            trim_trailing_whitespace,
            insert_final_newline,
            trim_final_newlines,
        ) = components.into_tuple();
        Self {
            tab_size,
            insert_spaces,
            properties: OrderMap::default(),
            trim_trailing_whitespace,
            insert_final_newline,
            trim_final_newlines,
        }
    }
}

compose! {
    DocumentOnTypeFormattingParams {
        text_document_position: TextDocumentPositionParams,
        ch: String,
        options: FormattingOptions
    }
}

impl crate::lsp::Compose for DocumentRangeFormattingParams {
    type Components = tuple_list_type![Selection, FormattingOptions, WorkDoneProgressParams];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (DocumentSelection(text_document, range), options, work_done_progress_params) =
            components.into_tuple();
        Self {
            text_document,
            range,
            options,
            work_done_progress_params,
        }
    }
}
