use lsp_types::{
    ClientCapabilities, CodeActionCapabilityResolveSupport, CodeActionClientCapabilities,
    CompletionClientCapabilities, CompletionItemCapability, CompletionItemCapabilityResolveSupport,
    CompletionItemKind, CompletionItemKindCapability, CompletionItemTag, CompletionListCapability,
    DiagnosticClientCapabilities, DiagnosticTag, DocumentLinkClientCapabilities,
    DocumentSymbolClientCapabilities, DynamicRegistrationClientCapabilities,
    FoldingRangeCapability, FoldingRangeClientCapabilities, FoldingRangeKind,
    GeneralClientCapabilities, GotoCapability, HoverClientCapabilities,
    InlayHintClientCapabilities, InsertTextMode, InsertTextModeSupport, MarkupKind,
    MessageActionItemCapabilities, ParameterInformationSettings, PositionEncodingKind,
    PublishDiagnosticsClientCapabilities, SelectionRangeClientCapabilities, SemanticTokenModifier,
    SemanticTokenType, SemanticTokensClientCapabilities, SemanticTokensClientCapabilitiesRequests,
    SemanticTokensFullOptions, ShowDocumentClientCapabilities,
    ShowMessageRequestClientCapabilities, SignatureHelpClientCapabilities,
    SignatureInformationSettings, SymbolKind, SymbolKindCapability, SymbolTag, TagSupport,
    TextDocumentClientCapabilities, TextDocumentSyncClientCapabilities, TokenFormat,
    WindowClientCapabilities, WorkspaceClientCapabilities, WorkspaceSymbolClientCapabilities,
    WorkspaceSymbolResolveSupportCapability,
};

pub fn fuzzer_client_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            workspace_folders: Some(true),
            symbol: Some(WorkspaceSymbolClientCapabilities {
                dynamic_registration: None,
                symbol_kind: Some(SymbolKindCapability {
                    value_set: Some(all_symbol_kinds()),
                }),
                tag_support: Some(TagSupport {
                    value_set: vec![SymbolTag::DEPRECATED],
                }),
                resolve_support: Some(WorkspaceSymbolResolveSupportCapability::default()),
            }),
            ..Default::default()
        }),
        text_document: Some(text_document_capabilities()),
        general: Some(GeneralClientCapabilities {
            position_encodings: Some(vec![PositionEncodingKind::UTF8]),
            ..Default::default()
        }),
        notebook_document: None,
        window: Some(WindowClientCapabilities {
            show_document: Some(ShowDocumentClientCapabilities { support: true }),
            show_message: Some(ShowMessageRequestClientCapabilities {
                message_action_item: Some(MessageActionItemCapabilities {
                    additional_properties_support: Some(true),
                }),
            }),
            work_done_progress: Some(true),
        }),
        experimental: None,
    }
}

fn text_document_capabilities() -> TextDocumentClientCapabilities {
    TextDocumentClientCapabilities {
        synchronization: Some(TextDocumentSyncClientCapabilities {
            ..Default::default()
        }),
        publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
            related_information: Some(true),
            tag_support: Some(TagSupport {
                value_set: vec![DiagnosticTag::UNNECESSARY, DiagnosticTag::DEPRECATED],
            }),
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
        completion: Some(completion_capabilities()),
        definition: Some(goto_capability()),
        type_definition: Some(goto_capability()),
        declaration: Some(goto_capability()),
        implementation: Some(goto_capability()),
        document_highlight: Some(DynamicRegistrationClientCapabilities::default()),
        references: Some(DynamicRegistrationClientCapabilities::default()),
        type_hierarchy: Some(DynamicRegistrationClientCapabilities::default()),
        call_hierarchy: Some(DynamicRegistrationClientCapabilities::default()),
        code_lens: Some(DynamicRegistrationClientCapabilities::default()),
        moniker: Some(DynamicRegistrationClientCapabilities::default()),
        color_provider: Some(DynamicRegistrationClientCapabilities::default()),
        on_type_formatting: Some(DynamicRegistrationClientCapabilities::default()),
        formatting: Some(DynamicRegistrationClientCapabilities::default()),
        range_formatting: Some(DynamicRegistrationClientCapabilities::default()),
        inline_value: Some(DynamicRegistrationClientCapabilities::default()),
        linked_editing_range: Some(DynamicRegistrationClientCapabilities::default()),
        code_action: Some(CodeActionClientCapabilities {
            dynamic_registration: None,
            code_action_literal_support: None,
            is_preferred_support: Some(true),
            disabled_support: Some(true),
            data_support: Some(true),
            resolve_support: Some(CodeActionCapabilityResolveSupport {
                properties: Vec::default(),
            }),
            honors_change_annotations: Some(true),
        }),
        document_symbol: Some(DocumentSymbolClientCapabilities {
            dynamic_registration: None,
            symbol_kind: Some(SymbolKindCapability {
                value_set: Some(all_symbol_kinds()),
            }),
            hierarchical_document_symbol_support: Some(true),
            tag_support: Some(TagSupport {
                value_set: vec![SymbolTag::DEPRECATED],
            }),
        }),
        document_link: Some(DocumentLinkClientCapabilities {
            dynamic_registration: None,
            tooltip_support: Some(true),
        }),
        signature_help: Some(SignatureHelpClientCapabilities {
            dynamic_registration: None,
            signature_information: Some(SignatureInformationSettings {
                documentation_format: Some(vec![MarkupKind::PlainText, MarkupKind::Markdown]),
                parameter_information: Some(ParameterInformationSettings {
                    label_offset_support: Some(true),
                }),
                active_parameter_support: Some(true),
            }),
            context_support: Some(true),
        }),
        rename: None,
        folding_range: Some(FoldingRangeClientCapabilities {
            dynamic_registration: None,
            range_limit: Some(1000),
            line_folding_only: Some(true),
            folding_range_kind: Some(lsp_types::FoldingRangeKindCapability {
                value_set: Some(vec![
                    FoldingRangeKind::Comment,
                    FoldingRangeKind::Imports,
                    FoldingRangeKind::Region,
                ]),
            }),
            folding_range: Some(FoldingRangeCapability {
                collapsed_text: Some(true),
            }),
        }),
        selection_range: Some(SelectionRangeClientCapabilities {
            dynamic_registration: None,
        }),
    }
}

const fn goto_capability() -> GotoCapability {
    GotoCapability {
        dynamic_registration: None,
        link_support: Some(true),
    }
}

fn all_symbol_kinds() -> Vec<SymbolKind> {
    vec![
        SymbolKind::FILE,
        SymbolKind::MODULE,
        SymbolKind::NAMESPACE,
        SymbolKind::PACKAGE,
        SymbolKind::CLASS,
        SymbolKind::METHOD,
        SymbolKind::PROPERTY,
        SymbolKind::FIELD,
        SymbolKind::CONSTRUCTOR,
        SymbolKind::ENUM,
        SymbolKind::INTERFACE,
        SymbolKind::FUNCTION,
        SymbolKind::VARIABLE,
        SymbolKind::CONSTANT,
        SymbolKind::STRING,
        SymbolKind::NUMBER,
        SymbolKind::BOOLEAN,
        SymbolKind::ARRAY,
        SymbolKind::OBJECT,
        SymbolKind::KEY,
        SymbolKind::NULL,
        SymbolKind::ENUM_MEMBER,
        SymbolKind::STRUCT,
        SymbolKind::EVENT,
        SymbolKind::OPERATOR,
        SymbolKind::TYPE_PARAMETER,
    ]
}

fn completion_capabilities() -> CompletionClientCapabilities {
    CompletionClientCapabilities {
        dynamic_registration: None,
        completion_item: Some(CompletionItemCapability {
            snippet_support: Some(true),
            commit_characters_support: Some(true),
            documentation_format: Some(vec![MarkupKind::PlainText, MarkupKind::Markdown]),
            deprecated_support: Some(true),
            preselect_support: Some(true),
            tag_support: Some(TagSupport {
                value_set: vec![CompletionItemTag::DEPRECATED],
            }),
            insert_replace_support: Some(true),
            resolve_support: Some(CompletionItemCapabilityResolveSupport::default()),
            insert_text_mode_support: Some(InsertTextModeSupport {
                value_set: vec![InsertTextMode::AS_IS, InsertTextMode::ADJUST_INDENTATION],
            }),
            label_details_support: Some(true),
        }),
        completion_item_kind: Some(CompletionItemKindCapability {
            value_set: Some(vec![
                CompletionItemKind::TEXT,
                CompletionItemKind::METHOD,
                CompletionItemKind::FUNCTION,
                CompletionItemKind::CONSTRUCTOR,
                CompletionItemKind::FIELD,
                CompletionItemKind::VARIABLE,
                CompletionItemKind::CLASS,
                CompletionItemKind::INTERFACE,
                CompletionItemKind::MODULE,
                CompletionItemKind::PROPERTY,
                CompletionItemKind::UNIT,
                CompletionItemKind::VALUE,
                CompletionItemKind::ENUM,
                CompletionItemKind::KEYWORD,
                CompletionItemKind::SNIPPET,
                CompletionItemKind::COLOR,
                CompletionItemKind::FILE,
                CompletionItemKind::REFERENCE,
                CompletionItemKind::FOLDER,
                CompletionItemKind::ENUM_MEMBER,
                CompletionItemKind::CONSTANT,
                CompletionItemKind::STRUCT,
                CompletionItemKind::EVENT,
                CompletionItemKind::OPERATOR,
                CompletionItemKind::TYPE_PARAMETER,
            ]),
        }),
        context_support: Some(true),
        insert_text_mode: Some(InsertTextMode::AS_IS),
        completion_list: Some(CompletionListCapability {
            item_defaults: Some(Vec::default()),
        }),
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
