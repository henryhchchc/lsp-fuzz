use serde::{Deserialize, Serialize};

macro_rules! lsp_requests {
    (
        $(#[$outer:meta])*
        $vis: vis enum $type_name: ident {
            $( $variant: ident, )*
        }
    ) => {
        use lsp_types::request::{self, Request};

        $(#[$outer])*
        $vis enum $type_name {
            $($variant(<request::$variant as Request>::Params),)*
        }

        impl $type_name {

            /// Returns the method name of the request.
            pub const fn method<'a>(&self) -> &'a str {
                match self {
                    $(Self::$variant(_) => <request::$variant as Request>::METHOD,)*
                }
            }

            /// Creates a JSON-RPC request object.
            pub fn as_json(&self, id: usize) -> serde_json::Value {
                match self {
                    $(
                        Self::$variant(params) => serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "method": <request::$variant as Request>::METHOD,
                            "params": params
                        }),
                    )*
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
        Initialize,
        Shutdown,
        ShowMessageRequest,
        RegisterCapability,
        UnregisterCapability,
        WorkspaceSymbolRequest,
        WorkspaceSymbolResolve,
        ExecuteCommand,
        WillSaveWaitUntil,
        Completion,
        ResolveCompletionItem,
        HoverRequest,
        SignatureHelpRequest,
        GotoDeclaration,
        GotoDefinition,
        References,
        DocumentHighlightRequest,
        DocumentSymbolRequest,
        CodeActionRequest,
        CodeLensRequest,
        CodeLensResolve,
        DocumentLinkRequest,
        DocumentLinkResolve,
        ApplyWorkspaceEdit,
        RangeFormatting,
        OnTypeFormatting,
        Formatting,
        Rename,
        DocumentColor,
        ColorPresentationRequest,
        FoldingRangeRequest,
        PrepareRenameRequest,
        GotoImplementation,
        GotoTypeDefinition,
        SelectionRangeRequest,
        WorkspaceFoldersRequest,
        WorkspaceConfiguration,
        WorkDoneProgressCreate,
        CallHierarchyIncomingCalls,
        CallHierarchyOutgoingCalls,
        MonikerRequest,
        LinkedEditingRange,
        CallHierarchyPrepare,
        TypeHierarchyPrepare,
        SemanticTokensFullRequest,
        SemanticTokensFullDeltaRequest,
        SemanticTokensRangeRequest,
        InlayHintRequest,
        InlineValueRequest,
        DocumentDiagnosticRequest,
        WorkspaceDiagnosticRequest,
        WorkspaceDiagnosticRefresh,
        TypeHierarchySupertypes,
        TypeHierarchySubtypes,
        WillCreateFiles,
        WillRenameFiles,
        WillDeleteFiles,
        SemanticTokensRefresh,
        CodeLensRefresh,
        InlayHintRefreshRequest,
        InlineValueRefreshRequest,
        CodeActionResolveRequest,
        InlayHintResolveRequest,
        ShowDocument,
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
