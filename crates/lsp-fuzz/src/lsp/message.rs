use std::mem;

use serde::{Deserialize, Serialize};

use super::json_rpc::JsonRPCMessage;
use crate::macros::{lsp_messages, lsp_responses};

lsp_messages! {
    /// A Language Server Protocol message.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[allow(clippy::large_enum_variant, reason = "By LSP spec")]
        pub enum LspMessage {
        // Client to Server messages
        request::CallHierarchyIncomingCalls,
        request::CallHierarchyOutgoingCalls,
        request::CallHierarchyPrepare,
        request::CodeActionRequest,
        request::CodeActionResolveRequest,
        request::CodeLensRequest,
        request::CodeLensResolve,
        request::ColorPresentationRequest,
        request::Completion,
        request::DocumentColor,
        request::DocumentDiagnosticRequest,
        request::DocumentHighlightRequest,
        request::DocumentLinkRequest,
        request::DocumentLinkResolve,
        request::DocumentSymbolRequest,
        request::ExecuteCommand,
        request::FoldingRangeRequest,
        request::Formatting,
        request::GotoDeclaration,
        request::GotoDefinition,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::HoverRequest,
        request::Initialize,
        request::InlayHintRequest,
        request::InlayHintResolveRequest,
        request::InlineValueRequest,
        request::LinkedEditingRange,
        request::MonikerRequest,
        request::OnTypeFormatting,
        request::PrepareRenameRequest,
        request::RangeFormatting,
        request::References,
        request::Rename,
        request::ResolveCompletionItem,
        request::SelectionRangeRequest,
        request::SemanticTokensFullDeltaRequest,
        request::SemanticTokensFullRequest,
        request::SemanticTokensRangeRequest,
        request::SemanticTokensRefresh,
        request::Shutdown,
        request::SignatureHelpRequest,
        request::TypeHierarchyPrepare,
        request::TypeHierarchySubtypes,
        request::TypeHierarchySupertypes,
        request::WillCreateFiles,
        request::WillDeleteFiles,
        request::WillRenameFiles,
        request::WillSaveWaitUntil,
        request::WorkspaceDiagnosticRefresh,
        request::WorkspaceDiagnosticRequest,
        request::WorkspaceSymbolRequest,
        request::WorkspaceSymbolResolve,
        // Server to Client messages
        request::ApplyWorkspaceEdit,
        request::CodeLensRefresh,
        request::InlayHintRefreshRequest,
        request::InlineValueRefreshRequest,
        request::RegisterCapability,
        request::ShowDocument,
        request::ShowMessageRequest,
        request::UnregisterCapability,
        request::WorkDoneProgressCreate,
        request::WorkspaceConfiguration,
        request::WorkspaceFoldersRequest,

        // Client to server notifications
        notification::Cancel,
        notification::DidChangeConfiguration,
        notification::DidChangeNotebookDocument,
        notification::DidChangeTextDocument,
        notification::DidChangeWatchedFiles,
        notification::DidChangeWorkspaceFolders,
        notification::DidCloseNotebookDocument,
        notification::DidCloseTextDocument,
        notification::DidCreateFiles,
        notification::DidDeleteFiles,
        notification::DidOpenNotebookDocument,
        notification::DidOpenTextDocument,
        notification::DidRenameFiles,
        notification::DidSaveNotebookDocument,
        notification::DidSaveTextDocument,
        notification::Exit,
        notification::Initialized,
        notification::LogTrace,
        notification::SetTrace,
        notification::WillSaveTextDocument,
        notification::WorkDoneProgressCancel,

        // Server to client notifications
        notification::LogMessage,
        notification::Progress,
        notification::PublishDiagnostics,
        notification::ShowMessage,
        notification::TelemetryEvent
    }
}

lsp_responses! {
    #[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
    #[allow(clippy::large_enum_variant, reason = "By LSP spec")]
    pub enum LspResponse {
        // Client to Server messages
        request::CallHierarchyIncomingCalls,
        request::CallHierarchyOutgoingCalls,
        request::CallHierarchyPrepare,
        request::CodeActionRequest,
        request::CodeActionResolveRequest,
        request::CodeLensRequest,
        request::CodeLensResolve,
        request::ColorPresentationRequest,
        request::Completion,
        request::DocumentColor,
        request::DocumentDiagnosticRequest,
        request::DocumentHighlightRequest,
        request::DocumentLinkRequest,
        request::DocumentLinkResolve,
        request::DocumentSymbolRequest,
        request::ExecuteCommand,
        request::FoldingRangeRequest,
        request::Formatting,
        request::GotoDeclaration,
        request::GotoDefinition,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::HoverRequest,
        request::Initialize,
        request::InlayHintRequest,
        request::InlayHintResolveRequest,
        request::InlineValueRequest,
        request::LinkedEditingRange,
        request::MonikerRequest,
        request::OnTypeFormatting,
        request::PrepareRenameRequest,
        request::RangeFormatting,
        request::References,
        request::Rename,
        request::ResolveCompletionItem,
        request::SelectionRangeRequest,
        request::SemanticTokensFullDeltaRequest,
        request::SemanticTokensFullRequest,
        request::SemanticTokensRangeRequest,
        request::SemanticTokensRefresh,
        request::Shutdown,
        request::SignatureHelpRequest,
        request::TypeHierarchyPrepare,
        request::TypeHierarchySubtypes,
        request::TypeHierarchySupertypes,
        request::WillCreateFiles,
        request::WillDeleteFiles,
        request::WillRenameFiles,
        request::WillSaveWaitUntil,
        request::WorkspaceDiagnosticRefresh,
        request::WorkspaceDiagnosticRequest,
        request::WorkspaceSymbolRequest,
        request::WorkspaceSymbolResolve,
        // Server to Client messages
        request::ApplyWorkspaceEdit,
        request::CodeLensRefresh,
        request::InlayHintRefreshRequest,
        request::InlineValueRefreshRequest,
        request::RegisterCapability,
        request::ShowDocument,
        request::ShowMessageRequest,
        request::UnregisterCapability,
        request::WorkDoneProgressCreate,
        request::WorkspaceConfiguration,
        request::WorkspaceFoldersRequest
    }
}

impl LspMessage {
    pub fn into_json_rpc(self, id: &mut usize) -> JsonRPCMessage {
        let is_request = self.is_request();
        let (method, params) = self.into_json();
        if is_request {
            let id = mem::replace(id, *id + 1);
            JsonRPCMessage::request(id, method.into(), params)
        } else {
            JsonRPCMessage::notification(method.into(), params)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessageDecodeError {
    #[error("Fail to deserialize the parameter {_0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("The message does not metch the expected type")]
    TypeMismatch,
    #[error("The message does not match the expected method")]
    MethodMismatch,
}

#[cfg(test)]
mod tests {
    use lsp_types::request::{HoverRequest, Request};

    use super::LspResponse;

    #[test]
    fn test_decode_response() {
        let response = serde_json::json!({
            "contents": { "kind": "markdown", "value": "**Documentation:** This is a test hover response" }
        });
        let LspResponse::HoverRequest(Some(response)) =
            LspResponse::try_from_json(HoverRequest::METHOD, response).unwrap()
        else {
            panic!("Response type mismatch")
        };
        assert!(response.range.is_none());
        assert_eq!(
            response.contents,
            lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: "**Documentation:** This is a test hover response".to_string()
            })
        );
    }
}
