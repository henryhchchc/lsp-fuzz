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

use super::{
    Compose,
    generation::{
        doc_range::Selection,
        numeric::{TabSize, ZeroToOne32},
    },
};
use crate::lsp_input::LspInput;

macro_rules! compose {
    ($output: ty {
        $( $field: ident: $field_type: ty ),*
    }) => {
        impl Compose for $output {
            type Components = tuple_list_type![
                $( $field_type ),*
            ];

            #[inline]
            fn compose(components: Self::Components) -> Self {
                let ( $( $field, )* ) = components.into_tuple();
                Self { $( $field ),* }
            }
        }
    };
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
compose! {
    FoldingRangeParams {
        text_document: TextDocumentIdentifier,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
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
    WorkspaceSymbolParams {
        query: String,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
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
    type Components = tuple_list_type![Selection, WorkDoneProgressParams,];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (Selection(text_document, range), work_done_progress_params) = components.into_tuple();
        Self {
            text_document,
            work_done_progress_params,
            range,
        }
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

impl Compose for SemanticTokensRangeParams {
    type Components = tuple_list_type![Selection, WorkDoneProgressParams, PartialResultParams];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (Selection(text_document, range), work_done_progress_params, partial_result_params) =
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
        Selection,
        WorkDoneProgressParams,
        PartialResultParams,
        CodeActionContext
    ];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (
            Selection(text_document, range),
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

compose! {
    LogTraceParams {
        message: String,
        verbose: Option<String>
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

compose! {
    RenameParams {
        text_document_position: TextDocumentPositionParams,
        new_name: String,
        work_done_progress_params: WorkDoneProgressParams
    }
}

impl Compose for ColorPresentationParams {
    type Components = tuple_list_type![
        Selection,
        Color,
        WorkDoneProgressParams,
        PartialResultParams
    ];

    fn compose(components: Self::Components) -> Self {
        let (
            Selection(text_document, range),
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

compose! {
    DocumentOnTypeFormattingParams {
        text_document_position: TextDocumentPositionParams,
        ch: String,
        options: FormattingOptions
    }
}

impl Compose for DocumentRangeFormattingParams {
    type Components = tuple_list_type![Selection, FormattingOptions, WorkDoneProgressParams];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (Selection(text_document, range), options, work_done_progress_params) =
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
