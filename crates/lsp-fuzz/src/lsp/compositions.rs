use lsp_types::{
    CallHierarchyPrepareParams, CodeActionContext, CodeActionKind, CodeActionParams,
    CodeActionTriggerKind, CodeLensParams, Color, ColorPresentationParams, CompletionContext,
    CompletionParams, CompletionTriggerKind, DocumentColorParams, DocumentDiagnosticParams,
    DocumentHighlightParams, DocumentLinkParams, DocumentOnTypeFormattingParams,
    DocumentRangeFormattingParams, DocumentSymbolParams, FoldingRangeParams, FormattingOptions,
    GotoDefinitionParams, HoverParams, InlayHintParams, LinkedEditingRangeParams, LogTraceParams,
    MonikerParams, PartialResultParams, PreviousResultId, ReferenceContext, ReferenceParams,
    RenameParams, SelectionRangeParams, SemanticTokensParams, SemanticTokensRangeParams,
    SignatureHelpContext, SignatureHelpParams, SignatureHelpTriggerKind, TextDocumentIdentifier,
    TextDocumentPositionParams, TypeHierarchyPrepareParams, WorkDoneProgressParams,
    WorkspaceDiagnosticParams, WorkspaceSymbolParams,
};
use trait_gen::trait_gen;
use tuple_list::{TupleList, tuple_list_type};

use crate::lsp_input::LspInput;

use super::{
    Compose,
    generation::{TabSize, ZeroToOne32, doc_range::RangeInDoc},
};

impl<Head, Tail> Compose for (Head, Tail) {
    type Components = (Head, Tail);

    #[inline]
    fn compose(components: Self::Components) -> Self {
        components
    }
}

impl Compose for FoldingRangeParams {
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
            text_document,
            work_done_progress_params,
            partial_result_params,
        }
    }
}

#[trait_gen(T ->
    GotoDefinitionParams,
    DocumentHighlightParams,
    MonikerParams,
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

impl Compose for SignatureHelpContext {
    type Components = tuple_list_type![
        SignatureHelpTriggerKind,
        Option<String>,
        bool,
        // TODO Option<SignatureHelp>
    ];

    fn compose(components: Self::Components) -> Self {
        let (
            trigger_kind,
            trigger_character,
            is_retrigger,
            // TODO active_signature_help
        ) = components.into_tuple();
        Self {
            trigger_kind,
            trigger_character,
            is_retrigger,
            active_signature_help: None,
        }
    }
}

impl Compose for SignatureHelpParams {
    type Components = tuple_list_type![
        Option<SignatureHelpContext>,
        TextDocumentPositionParams,
        WorkDoneProgressParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (context, text_document_position_params, work_done_progress_params) =
            components.into_tuple();
        Self {
            context,
            text_document_position_params,
            work_done_progress_params,
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
    type Components = tuple_list_type![RangeInDoc, WorkDoneProgressParams,];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (RangeInDoc(text_document, range), work_done_progress_params) = components.into_tuple();
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

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (trigger_kind, trigger_character) = components.into_tuple();
        Self {
            trigger_kind,
            trigger_character,
        }
    }
}

impl Compose for SemanticTokensRangeParams {
    type Components = tuple_list_type![RangeInDoc, WorkDoneProgressParams, PartialResultParams];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (RangeInDoc(text_document, range), work_done_progress_params, partial_result_params) =
            components.into_tuple();
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
        RangeInDoc,
        WorkDoneProgressParams,
        PartialResultParams,
        CodeActionContext
    ];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (
            RangeInDoc(text_document, range),
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
    type Components = tuple_list_type![Option<Vec<CodeActionKind>>, Option<CodeActionTriggerKind>];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (only, trigger_kind) = components.into_tuple();
        Self {
            // TODO: Implement this
            diagnostics: vec![],
            only,
            trigger_kind,
        }
    }
}

impl Compose for LogTraceParams {
    type Components = tuple_list_type![String, Option<String>];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (message, verbose) = components.into_tuple();
        Self { message, verbose }
    }
}

impl Compose for WorkspaceDiagnosticParams {
    type Components = tuple_list_type![
        Option<String>,
        Vec<PreviousResultId>,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (identifier, previous_result_ids, work_done_progress_params, partial_result_params) =
            components.into_tuple();
        Self {
            identifier,
            previous_result_ids,
            work_done_progress_params,
            partial_result_params,
        }
    }
}

impl Compose for PreviousResultId {
    type Components = tuple_list_type![String];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (value,) = components.into_tuple();
        Self {
            uri: LspInput::root_uri(),
            value,
        }
    }
}

impl Compose for RenameParams {
    type Components = tuple_list_type![TextDocumentPositionParams, String, WorkDoneProgressParams,];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (text_document_position, new_name, work_done_progress_params) = components.into_tuple();
        Self {
            text_document_position,
            new_name,
            work_done_progress_params,
        }
    }
}

impl Compose for ColorPresentationParams {
    type Components = tuple_list_type![
        RangeInDoc,
        Color,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (
            RangeInDoc(text_document, range),
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

impl Compose for Color {
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

impl Compose for FormattingOptions {
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
            properties: Default::default(),
            trim_trailing_whitespace,
            insert_final_newline,
            trim_final_newlines,
        }
    }
}

impl Compose for DocumentOnTypeFormattingParams {
    type Components = tuple_list_type![TextDocumentPositionParams, String, FormattingOptions];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (text_document_position, ch, options) = components.into_tuple();
        Self {
            text_document_position,
            ch,
            options,
        }
    }
}

impl Compose for DocumentRangeFormattingParams {
    type Components = tuple_list_type![RangeInDoc, FormattingOptions, WorkDoneProgressParams];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (RangeInDoc(text_document, range), options, work_done_progress_params) =
            components.into_tuple();
        Self {
            text_document,
            range,
            options,
            work_done_progress_params,
        }
    }
}

impl Compose for SelectionRangeParams {
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
