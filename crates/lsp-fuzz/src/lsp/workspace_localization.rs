use std::any::type_name;

use crate::macros::impl_localize;

use super::LocalizeToWorkspace;
use lsp_types::*;
use lsp_types::{
    CompletionParams, DidOpenTextDocumentParams, GotoDefinitionParams, HoverParams,
    InitializeParams, InlayHintParams, OneOf, SemanticTokensParams, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, WorkspaceFolder,
};
use trait_gen::trait_gen;

#[trait_gen(T ->
    ApplyWorkspaceEditParams,
    CallHierarchyIncomingCallsParams,
    CallHierarchyOutgoingCallsParams,
    CallHierarchyPrepareParams,
    CancelParams,
    CodeAction,
    CodeActionParams,
    CodeLens,
    CodeLensParams,
    ColorPresentationParams,
    CompletionItem,
    ConfigurationParams,
    CreateFilesParams,
    DeleteFilesParams,
    DidChangeConfigurationParams,
    DidChangeNotebookDocumentParams,
    DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams,
    DidCloseNotebookDocumentParams,
    DidCloseTextDocumentParams,
    DidOpenNotebookDocumentParams,
    DidSaveNotebookDocumentParams,
    DidSaveTextDocumentParams,
    DocumentColorParams,
    DocumentDiagnosticParams,
    DocumentFormattingParams,
    DocumentLink,
    DocumentLinkParams,
    DocumentOnTypeFormattingParams,
    DocumentRangeFormattingParams,
    DocumentSymbolParams,
    ExecuteCommandParams,
    FoldingRangeParams,
    InitializeResult,
    InitializedParams,
    InlayHint,
    InlineValueParams,
    LinkedEditingRangeParams,
    LogMessageParams,
    LogTraceParams,
    MonikerParams,
    ProgressParams,
    PublishDiagnosticsParams,
    RegistrationParams,
    RenameFilesParams,
    RenameParams,
    SelectionRangeParams,
    SemanticTokensDeltaParams,
    SemanticTokensRangeParams,
    SetTraceParams,
    ShowDocumentParams,
    ShowMessageParams,
    ShowMessageRequestParams,
    SignatureHelpParams,
    TypeHierarchySubtypesParams,
    TypeHierarchySupertypesParams,
    UnregistrationParams,
    WillSaveTextDocumentParams,
    WorkDoneProgressCancelParams,
    WorkDoneProgressCreateParams,
    WorkDoneProgressParams,
    WorkspaceDiagnosticParams,
    WorkspaceSymbol,
    WorkspaceSymbolParams,
)]
impl LocalizeToWorkspace for T {
    fn localize(self, _workspace_dir: &str) -> Self {
        todo!(
            "Ouch! LocalizeToWorkspace not (meaningfully) implemented for {}",
            type_name::<Self>()
        );
    }
}

#[trait_gen(T ->
    (),
    serde_json::Map<String, serde_json::Value>,
    serde_json::Value
)]
impl LocalizeToWorkspace for T {
    fn localize(self, _workspace_dir: &str) -> Self {
        self
    }
}

impl<A, B> LocalizeToWorkspace for OneOf<A, B>
where
    A: LocalizeToWorkspace,
    B: LocalizeToWorkspace,
{
    fn localize(self, workspace_dir: &str) -> Self {
        match self {
            Self::Left(lhs) => Self::Left(lhs.localize(workspace_dir)),
            Self::Right(rhs) => Self::Right(rhs.localize(workspace_dir)),
        }
    }
}

impl<T> LocalizeToWorkspace for Option<T>
where
    T: LocalizeToWorkspace,
{
    fn localize(self, workspace_dir: &str) -> Self {
        self.map(|it| it.localize(workspace_dir))
    }
}

impl<T> LocalizeToWorkspace for Vec<T>
where
    T: LocalizeToWorkspace,
{
    fn localize(self, workspace_dir: &str) -> Self {
        self.into_iter()
            .map(|it| it.localize(workspace_dir))
            .collect()
    }
}

impl LocalizeToWorkspace for lsp_types::Uri {
    fn localize(self, workspace_dir: &str) -> Self {
        format!(
            "file://{}/{}",
            workspace_dir,
            self.to_string().strip_prefix("lsp-fuzz://").unwrap()
        )
        .parse()
        .unwrap()
    }
}

impl_localize!(CompletionParams; text_document_position);
impl_localize!(DidOpenTextDocumentParams; text_document);
impl_localize!(GotoDefinitionParams; text_document_position_params);
impl_localize!(HoverParams; text_document_position_params);
impl_localize!(InitializeParams; workspace_folders);
impl_localize!(InlayHintParams; text_document);
impl_localize!(SemanticTokensParams; text_document);
impl_localize!(TextDocumentIdentifier; uri);
impl_localize!(TextDocumentItem; uri);
impl_localize!(TextDocumentPositionParams; text_document);
impl_localize!(WorkspaceFolder; uri);
impl_localize!(TypeHierarchyPrepareParams; text_document_position_params);
impl_localize!(ReferenceParams; text_document_position);
impl_localize!(DocumentHighlightParams; text_document_position_params);
