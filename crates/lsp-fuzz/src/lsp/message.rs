use std::{borrow::Cow, mem, ops::Range};

use serde::{Deserialize, Serialize};

use super::json_rpc::JsonRPCMessage;
use crate::{lsp_input::LspInput, macros::lsp_messages};

lsp_messages! {
    /// A Language Server Protocol message.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[allow(clippy::large_enum_variant, reason = "By LSP spec")]
        pub enum ClientToServerMessage {
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
        // request::ApplyWorkspaceEdit,
        // request::CodeLensRefresh,
        // request::InlayHintRefreshRequest,
        // request::InlineValueRefreshRequest,
        // request::RegisterCapability,
        // request::ShowDocument,
        // request::ShowMessageRequest,
        // request::UnregisterCapability,
        // request::WorkDoneProgressCreate,
        // request::WorkspaceConfiguration,
        // request::WorkspaceFoldersRequest,

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
        // notification::LogMessage,
        // notification::Progress,
        // notification::PublishDiagnostics,
        // notification::ShowMessage,
        // notification::TelemetryEvent,
    }
}

impl ClientToServerMessage {
    pub fn into_json_rpc(self, id: &mut usize, workspace_uri: Option<&str>) -> JsonRPCMessage {
        let is_request = self.is_request();
        let (method, mut params) = self.into_json();
        if let Some(workspace_uri) = workspace_uri {
            let workspace_uri = if workspace_uri.ends_with('/') {
                Cow::Borrowed(workspace_uri)
            } else {
                Cow::Owned(format!("{workspace_uri}/"))
            };
            localize_json_value(&mut params, workspace_uri.as_ref());
        }
        if is_request {
            let id = mem::replace(id, *id + 1);
            JsonRPCMessage::request(id, method.into(), params)
        } else {
            JsonRPCMessage::notification(method.into(), params)
        }
    }
}

fn localize_json_value(value: &mut serde_json::Value, workspace_uri: &str) {
    use serde_json::Value::{Array, Object, String};
    const LSP_FUZZ_PREFIX_RANGE: Range<usize> = 0..LspInput::PROROCOL_PREFIX.len();
    match value {
        Object(inner) => inner.values_mut().for_each(|value| {
            localize_json_value(value, workspace_uri);
        }),
        Array(items) => items.iter_mut().for_each(|value| {
            localize_json_value(value, workspace_uri);
        }),
        String(str_val) if str_val.starts_with(LspInput::PROROCOL_PREFIX) => {
            str_val.replace_range(LSP_FUZZ_PREFIX_RANGE, workspace_uri)
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_localization() {
        let mut value = serde_json::json!({
            "uri": "lsp-fuzz://path/to/file",
            "other_attr": {
                "uri": "lsp-fuzz://path/to/other_file"
            },
            "some_arr": [
                "lsp-fuzz://path/to/element",
            ],
            "other_arr": [
                {
                    "uri": "lsp-fuzz://path/to/element",
                }
            ]
        });
        super::localize_json_value(&mut value, "file:///path/to/workspace_dir/");
        assert_eq!(
            value,
            serde_json::json!({
                "uri": "file:///path/to/workspace_dir/path/to/file",
                "other_attr": {
                    "uri": "file:///path/to/workspace_dir/path/to/other_file"
                },
                "some_arr": [
                    "file:///path/to/workspace_dir/path/to/element",
                ],
                "other_arr": [
                    {
                        "uri": "file:///path/to/workspace_dir/path/to/element",
                    }
                ]
            })
        );
    }
}
