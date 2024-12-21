use std::any::type_name;

use crate::macros::impl_localize;

use super::LocalizeToWorkspace;
use lsp_types::*;
use lsp_types::{
    CompletionParams, DidOpenTextDocumentParams, GotoDefinitionParams, HoverParams,
    InitializeParams, InlayHintParams, OneOf, SemanticTokensParams, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, WorkspaceFolder,
};
use ordermap::OrderMap;
use trait_gen::trait_gen;

#[trait_gen(T ->
    CallHierarchyIncomingCallsParams,
    CallHierarchyOutgoingCallsParams,
    CodeAction,
    CodeActionParams,
    CodeLens,
    ColorPresentationParams,
    CompletionItem,
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
    DocumentLink,
    DocumentOnTypeFormattingParams,
    DocumentRangeFormattingParams,
    ExecuteCommandParams,
    FoldingRangeParams,
    InitializeResult,
    InlayHint,
    LinkedEditingRangeParams,
    LogTraceParams,
    MonikerParams,
    ProgressParams,
    PublishDiagnosticsParams,
    RegistrationParams,
    RenameFilesParams,
    RenameParams,
    SelectionRangeParams,
    SetTraceParams,
    ShowDocumentParams,
    TypeHierarchySubtypesParams,
    TypeHierarchySupertypesParams,
    WillSaveTextDocumentParams,
    WorkspaceDiagnosticParams,
)]
impl LocalizeToWorkspace for T {
    fn localize(&mut self, _workspace_dir: &str) {
        todo!(
            "Generic LocalizeToWorkspace should not be used. \
            This trait should be meaningfully implemented for {}",
            type_name::<Self>()
        );
    }
}

#[trait_gen(T ->
    (),
    serde_json::Map<String, serde_json::Value>,
    serde_json::Value,
    CancelParams,
    WorkDoneProgressParams,
    WorkDoneProgressCancelParams,
    WorkDoneProgressCreateParams,
    UnregistrationParams,
    ShowMessageParams,
    ShowMessageRequestParams,
    TextEdit,
    LogMessageParams,
    WorkspaceSymbolParams,
    InitializedParams
)]
impl LocalizeToWorkspace for T {
    #[inline]
    fn localize(&mut self, _workspace_dir: &str) {}
}

impl<A, B> LocalizeToWorkspace for OneOf<A, B>
where
    A: LocalizeToWorkspace,
    B: LocalizeToWorkspace,
{
    fn localize(&mut self, workspace_dir: &str) {
        match self {
            Self::Left(lhs) => lhs.localize(workspace_dir),
            Self::Right(rhs) => rhs.localize(workspace_dir),
        }
    }
}

impl<T> LocalizeToWorkspace for Option<T>
where
    T: LocalizeToWorkspace,
{
    fn localize(&mut self, workspace_dir: &str) {
        if let Some(inner) = self {
            inner.localize(workspace_dir);
        }
    }
}

impl<K, V> LocalizeToWorkspace for OrderMap<K, V>
where
    K: LocalizeToWorkspace + Eq + std::hash::Hash,
    V: LocalizeToWorkspace,
{
    fn localize(&mut self, workspace_dir: &str) {
        let mut new = OrderMap::new();
        for (mut k, mut v) in self.drain(..) {
            k.localize(workspace_dir);
            v.localize(workspace_dir);
            new.insert(k, v);
        }
        *self = new;
    }
}

impl<T> LocalizeToWorkspace for Vec<T>
where
    T: LocalizeToWorkspace,
{
    fn localize(&mut self, workspace_dir: &str) {
        self.iter_mut().for_each(|it| it.localize(workspace_dir))
    }
}

impl LocalizeToWorkspace for lsp_types::Uri {
    fn localize(&mut self, workspace_dir: &str) {
        *self = format!(
            "file://{}/{}",
            workspace_dir,
            self.to_string().strip_prefix("lsp-fuzz://").unwrap()
        )
        .parse()
        .unwrap();
    }
}

impl_localize!(CompletionParams; text_document_position);
impl_localize!(InitializeParams; workspace_folders);
impl_localize!(ReferenceParams; text_document_position);
impl_localize!(ApplyWorkspaceEditParams; edit);
impl_localize!(WorkspaceEdit; changes, document_changes);
impl_localize!(RenameFile; old_uri, new_uri);
impl_localize!(ConfigurationParams; items);
impl_localize!(ConfigurationItem; scope_uri);
impl_localize!(WorkspaceSymbol; location);

impl LocalizeToWorkspace for DocumentChangeOperation {
    #[inline]
    fn localize(&mut self, workspace_dir: &str) {
        match self {
            Self::Edit(inner) => inner.localize(workspace_dir),
            Self::Op(inner) => inner.localize(workspace_dir),
        }
    }
}

#[trait_gen(T ->
    TextDocumentIdentifier,
    TextDocumentItem,
    OptionalVersionedTextDocumentIdentifier,
    WorkspaceFolder,
    CreateFile,
    DeleteFile,
    Location,
    WorkspaceLocation,
)]
impl LocalizeToWorkspace for T {
    #[inline]
    fn localize(&mut self, workspace_dir: &str) {
        self.uri.localize(workspace_dir);
    }
}

impl LocalizeToWorkspace for ResourceOp {
    #[inline]
    fn localize(&mut self, workspace_dir: &str) {
        match self {
            Self::Create(inner) => inner.localize(workspace_dir),
            Self::Delete(inner) => inner.localize(workspace_dir),
            Self::Rename(inner) => inner.localize(workspace_dir),
        }
    }
}

impl LocalizeToWorkspace for DocumentChanges {
    #[inline]
    fn localize(&mut self, workspace_dir: &str) {
        match self {
            Self::Edits(inner) => inner.localize(workspace_dir),
            Self::Operations(inner) => inner.localize(workspace_dir),
        }
    }
}

#[trait_gen(T ->
    InlayHintParams,
    SemanticTokensParams,
    SemanticTokensRangeParams,
    SemanticTokensDeltaParams,
    TextDocumentPositionParams,
    DidOpenTextDocumentParams,
    TextDocumentEdit,
    InlineValueParams,
    DocumentDiagnosticParams,
    DocumentColorParams,
    DocumentFormattingParams,
    CodeLensParams,
    DocumentSymbolParams,
    DocumentLinkParams,
)]
impl LocalizeToWorkspace for T {
    #[inline]
    fn localize(&mut self, workspace_dir: &str) {
        self.text_document.localize(workspace_dir);
    }
}

#[trait_gen(T ->
    CallHierarchyPrepareParams,
    GotoDefinitionParams,
    HoverParams,
    SignatureHelpParams,
    TypeHierarchyPrepareParams,
    DocumentHighlightParams,
)]
impl LocalizeToWorkspace for T {
    #[inline]
    fn localize(&mut self, workspace_dir: &str) {
        self.text_document_position_params.localize(workspace_dir);
    }
}
