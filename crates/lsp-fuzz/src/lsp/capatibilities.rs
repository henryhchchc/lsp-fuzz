use lsp_types::{
    ClientCapabilities, DiagnosticClientCapabilities, GeneralClientCapabilities,
    HoverClientCapabilities, InlayHintClientCapabilities, MarkupKind,
    PublishDiagnosticsClientCapabilities, SemanticTokenModifier, SemanticTokenType,
    SemanticTokensClientCapabilities, SemanticTokensClientCapabilitiesRequests,
    SemanticTokensFullOptions, TextDocumentClientCapabilities, TextDocumentSyncClientCapabilities,
    TokenFormat, WindowClientCapabilities, WorkspaceClientCapabilities,
};

pub fn fuzzer_client_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            workspace_folders: Some(true),
            ..Default::default()
        }),
        text_document: Some(TextDocumentClientCapabilities {
            synchronization: Some(TextDocumentSyncClientCapabilities {
                ..Default::default()
            }),
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                related_information: Some(true),
                tag_support: None,
                version_support: Some(true),
                code_description_support: Some(true),
                data_support: Some(true),
            }),
            diagnostic: Some(DiagnosticClientCapabilities {
                dynamic_registration: None,
                related_document_support: Some(true),
            }),
            inlay_hint: Some(InlayHintClientCapabilities::default()),
            hover: Some(HoverClientCapabilities {
                content_format: Some(vec![MarkupKind::PlainText, MarkupKind::Markdown]),
                dynamic_registration: None,
            }),
            semantic_tokens: Some(full_semantic_tokens_client_capabilities()),
            ..Default::default()
        }),
        general: Some(GeneralClientCapabilities {
            ..Default::default()
        }),
        notebook_document: None,
        window: Some(WindowClientCapabilities {
            ..Default::default()
        }),
        experimental: None,
    }
}

fn full_semantic_tokens_client_capabilities() -> SemanticTokensClientCapabilities {
    SemanticTokensClientCapabilities {
        dynamic_registration: None,
        requests: SemanticTokensClientCapabilitiesRequests {
            range: Some(true),
            full: Some(SemanticTokensFullOptions::Bool(true)),
        },
        token_types: vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::EVENT,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::MODIFICATION,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ],
        formats: vec![TokenFormat::RELATIVE],
        overlapping_token_support: Some(true),
        multiline_token_support: Some(true),
        server_cancel_support: Some(true),
        augments_syntax_tokens: Some(true),
    }
}
