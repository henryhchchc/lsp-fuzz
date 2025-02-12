use std::{mem, ops::Range};

use serde::{Deserialize, Serialize};

use crate::macros::lsp_messages;

use super::json_rpc::JsonRPCMessage;

lsp_messages! {
    /// A Language Server Protocol message.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[allow(clippy::large_enum_variant, reason = "By LSP spec")]
    pub enum ClientToServerMessage {
        request::Initialize,
        request::Shutdown,
        // request::ShowMessageRequest,
        // request::RegisterCapability,
        // request::UnregisterCapability,
        request::WorkspaceSymbolRequest,
        request::WorkspaceSymbolResolve,
        request::ExecuteCommand,
        request::WillSaveWaitUntil,
        request::Completion,
        request::ResolveCompletionItem,
        request::HoverRequest,
        request::SignatureHelpRequest,
        request::GotoDeclaration,
        request::GotoDefinition,
        request::References,
        request::DocumentHighlightRequest,
        request::DocumentSymbolRequest,
        request::CodeActionRequest,
        request::CodeLensRequest,
        request::CodeLensResolve,
        request::DocumentLinkRequest,
        request::DocumentLinkResolve,
        // request::ApplyWorkspaceEdit,
        request::RangeFormatting,
        request::OnTypeFormatting,
        // request::Formatting,
        request::Rename,
        request::DocumentColor,
        request::ColorPresentationRequest,
        request::FoldingRangeRequest,
        request::PrepareRenameRequest,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::SelectionRangeRequest,
        // request::WorkspaceFoldersRequest,
        // request::WorkspaceConfiguration,
        // request::WorkDoneProgressCreate,
        request::CallHierarchyIncomingCalls,
        request::CallHierarchyOutgoingCalls,
        request::MonikerRequest,
        request::LinkedEditingRange,
        request::CallHierarchyPrepare,
        request::TypeHierarchyPrepare,
        request::SemanticTokensFullRequest,
        request::SemanticTokensFullDeltaRequest,
        request::SemanticTokensRangeRequest,
        request::InlayHintRequest,
        request::InlineValueRequest,
        request::DocumentDiagnosticRequest,
        request::WorkspaceDiagnosticRequest,
        request::WorkspaceDiagnosticRefresh,
        request::TypeHierarchySupertypes,
        request::TypeHierarchySubtypes,
        request::WillCreateFiles,
        request::WillRenameFiles,
        request::WillDeleteFiles,
        request::SemanticTokensRefresh,
        // request::CodeLensRefresh,
        // request::InlayHintRefreshRequest,
        // request::InlineValueRefreshRequest,
        request::CodeActionResolveRequest,
        request::InlayHintResolveRequest,
        // request::ShowDocument,
        notification::Cancel,
        notification::SetTrace,
        notification::LogTrace,
        notification::Initialized,
        notification::Exit,
        // notification::ShowMessage,
        // notification::LogMessage,
        notification::WorkDoneProgressCancel,
        // notification::TelemetryEvent,
        notification::DidOpenTextDocument,
        notification::DidChangeTextDocument,
        notification::WillSaveTextDocument,
        notification::DidSaveTextDocument,
        notification::DidCloseTextDocument,
        // notification::PublishDiagnostics,
        notification::DidOpenNotebookDocument,
        notification::DidChangeNotebookDocument,
        notification::DidSaveNotebookDocument,
        notification::DidCloseNotebookDocument,
        notification::DidChangeConfiguration,
        notification::DidChangeWatchedFiles,
        notification::DidChangeWorkspaceFolders,
        // notification::Progress,
        notification::DidCreateFiles,
        notification::DidRenameFiles,
        notification::DidDeleteFiles
    }
}

impl ClientToServerMessage {
    pub fn into_json_rpc(self, id: &mut usize, localize: Option<&str>) -> JsonRPCMessage {
        let is_request = self.is_request();
        let (method, mut params) = self.into_json();
        if let Some(workspace_uri) = localize {
            localize_json_value(&mut params, workspace_uri);
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
    assert!(workspace_uri.ends_with('/'));
    use serde_json::Value::{Array, Object, String};
    const LSP_FUZZ_PREFIX: &str = "lsp-fuzz://";
    const LSP_FUZZ_PREFIX_RANGE: Range<usize> = 0..LSP_FUZZ_PREFIX.len();
    match value {
        Object(inner) => inner.iter_mut().for_each(|(_, v)| {
            localize_json_value(v, workspace_uri);
        }),
        Array(items) => items.iter_mut().for_each(|item| {
            localize_json_value(item, workspace_uri);
        }),
        String(str_val) if str_val.starts_with(LSP_FUZZ_PREFIX) => {
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
