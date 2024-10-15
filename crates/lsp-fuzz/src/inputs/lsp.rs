use serde::{Deserialize, Serialize};

macro_rules! lsp_requests {
    (
        $(#[$outer:meta])*
        $vis: vis enum $type_name: ident {
            $(
                $( request::$req_variant: ident )?
                $( notification::$not_variant: ident )?
            ),*
        }
    ) => {
        use lsp_types::request::{self, Request};
        use lsp_types::notification::{self, Notification};

        $(#[$outer])*
        $vis enum $type_name {
            $(
                $( $req_variant(<request::$req_variant as Request>::Params) )?
                $( $not_variant(<notification::$not_variant as Notification>::Params) )?
            ),*
        }

        impl $type_name {

            /// Returns the method name of the request.
            pub const fn method<'a>(&self) -> &'a str {
                match self {
                    $(
                        $( Self::$req_variant(_) => <request::$req_variant as Request>::METHOD )?
                        $( Self::$not_variant(_) => <notification::$not_variant as Notification>::METHOD )?
                    ),*
                }
            }

            /// Creates a JSON-RPC request object.
            pub fn as_json(&self, id: usize) -> serde_json::Value {
                match self {
                    $(
                        $(
                            Self::$req_variant(params) => serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "method": <request::$req_variant as Request>::METHOD,
                                "params": params
                            })
                        )?
                        $(
                            Self::$not_variant(params) => serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "method": <notification::$not_variant as Notification>::METHOD,
                                "params": params
                            })
                        )?
                    ),*
                }
            }

        }
    };
}

lsp_requests! {

    /// A Language Server Protocol request.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[allow(clippy::large_enum_variant, reason = "By LSP spec")]
    pub enum LspRequest {
        request::Initialize,
        request::Shutdown,
        request::ShowMessageRequest,
        request::RegisterCapability,
        request::UnregisterCapability,
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
        request::ApplyWorkspaceEdit,
        request::RangeFormatting,
        request::OnTypeFormatting,
        request::Formatting,
        request::Rename,
        request::DocumentColor,
        request::ColorPresentationRequest,
        request::FoldingRangeRequest,
        request::PrepareRenameRequest,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::SelectionRangeRequest,
        request::WorkspaceFoldersRequest,
        request::WorkspaceConfiguration,
        request::WorkDoneProgressCreate,
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
        request::CodeLensRefresh,
        request::InlayHintRefreshRequest,
        request::InlineValueRefreshRequest,
        request::CodeActionResolveRequest,
        request::InlayHintResolveRequest,
        request::ShowDocument,
        notification::Cancel,
        notification::SetTrace,
        notification::LogTrace,
        notification::Initialized,
        notification::Exit,
        notification::ShowMessage,
        notification::LogMessage,
        notification::WorkDoneProgressCancel,
        notification::TelemetryEvent,
        notification::DidOpenTextDocument,
        notification::DidChangeTextDocument,
        notification::WillSaveTextDocument,
        notification::DidSaveTextDocument,
        notification::DidCloseTextDocument,
        notification::PublishDiagnostics,
        notification::DidOpenNotebookDocument,
        notification::DidChangeNotebookDocument,
        notification::DidSaveNotebookDocument,
        notification::DidCloseNotebookDocument,
        notification::DidChangeConfiguration,
        notification::DidChangeWatchedFiles,
        notification::DidChangeWorkspaceFolders,
        notification::Progress,
        notification::DidCreateFiles,
        notification::DidRenameFiles,
        notification::DidDeleteFiles
    }
}

pub fn encapsulate_request_content(request_object: &serde_json::Value) -> Vec<u8> {
    let request_body =
        serde_json::to_vec(&request_object).expect("JSON value must be serializable to bytes");
    let content_length = request_body.len();
    let mut result = format!("Content-Length: {content_length}\r\n\r\n").into_bytes();
    result.extend(request_body);
    result
}

#[cfg(test)]
mod test {
    use lsp_types::request::{Initialize, Request};

    use crate::inputs::lsp::encapsulate_request_content;

    use super::LspRequest;

    #[test]
    fn test_lsp_request() {
        let request = LspRequest::Initialize(lsp_types::InitializeParams {
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: "file:///path/to/folder".parse().unwrap(),
                name: "folder".to_string(),
            }]),
            ..Default::default()
        });
        let jsonrpc = encapsulate_request_content(&request.as_json(1));
        let header = b"Content-Length: 177\r\n\r\n";
        assert_eq!(jsonrpc[..header.len()], header[..]);
        let json_value: serde_json::Value =
            serde_json::from_slice(&jsonrpc[header.len()..]).unwrap();
        assert_eq!(json_value["jsonrpc"], "2.0");
        assert_eq!(json_value["id"], 1);
        assert_eq!(json_value["method"], Initialize::METHOD);
        assert!(json_value["params"]["workspaceFolders"][0]["uri"]
            .as_str()
            .unwrap()
            .contains("path/to/folder"));
    }
}
