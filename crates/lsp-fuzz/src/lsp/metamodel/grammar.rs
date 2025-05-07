use libafl::nautilus::grammartec::context::Context;

pub fn get_nautilus_context() -> Context {
    let rules = get_grammar_rules();
    let mut grammar = Context::new();
    for (nt, rule) in rules {
        grammar.add_rule(nt, &rule);
    }
    grammar
}

#[test]
fn test_grammar() {
    let rules = get_grammar_rules();
    eprintln!("{} production rules.", rules.len());
    let mut grammar = Context::new();
    for (nt, rule) in rules {
        grammar.add_rule(nt, &rule);
    }
    grammar.initialize(100_000);
}

fn get_grammar_rules() -> Vec<(&'static str, Vec<u8>)> {
    let mut rules: Vec<(&'static str, Vec<u8>)> = vec![];

    let mut add_rule = |nt, format: &[u8]| {
        rules.push((nt, format.to_vec()));
    };

    // Core message structure
    add_rule("START", b"{REQUEST}");
    add_rule("START", b"{NOTIFICATION}");

    // Numbers
    add_rule("NUMBER", b"{DIGIT}");
    add_rule("NUMBER", b"{DIGIT}{NUMBER}");
    add_rule("NUMBER", b"-{NUMBER}"); // Allow negative numbers
    add_rule("NUMBER", b"{NUMBER}.{NUMBER}"); // Allow decimal numbers
    for i in 0..=9 {
        add_rule("DIGIT", &[b'0' + i]);
    }

    // Strings
    add_rule("STRING", b"\"{CHAR}\"");
    add_rule("STRING", b"{CHAR}{STRING}");
    add_rule("CHAR", b"{DIGIT}");
    for letter in b'a'..=b'z' {
        add_rule("CHAR", &[letter]);
    }
    for letter in b'A'..=b'Z' {
        add_rule("CHAR", &[letter]);
    }
    add_rule("CHAR", b"_");
    add_rule("CHAR", b"-");
    add_rule("CHAR", b"/");
    add_rule("CHAR", b".");
    add_rule("CHAR", b",");
    add_rule("CHAR", b":");
    add_rule("CHAR", b"$");
    add_rule("CHAR", b"@");
    add_rule("CHAR", b"#");
    add_rule("CHAR", b"!");
    add_rule("CHAR", b"?");
    add_rule("CHAR", b"+");
    add_rule("CHAR", b"*");
    add_rule("CHAR", b"&");
    add_rule("CHAR", b"%");
    add_rule("CHAR", b"=");
    add_rule("CHAR", b" ");

    // JSON Object
    add_rule("JSON_OBJECT", b"\\{\\}");
    add_rule("JSON_OBJECT", b"\\{{JSON_MEMBERS}\\}");
    add_rule("JSON_MEMBERS", b"{JSON_MEMBER}");
    add_rule("JSON_MEMBERS", b"{JSON_MEMBER},{JSON_MEMBERS}");
    add_rule("JSON_MEMBER", b"\"{STRING_CONTENT}\":{JSON_VALUE}");

    // JSON Array
    add_rule("JSON_ARRAY", b"\\[\\]");
    add_rule("JSON_ARRAY", b"\\[{JSON_ELEMENTS}\\]");
    add_rule("JSON_ELEMENTS", b"{JSON_VALUE}");
    add_rule("JSON_ELEMENTS", b"{JSON_VALUE},{JSON_ELEMENTS}");

    // JSON Value
    add_rule("JSON_VALUE", b"{JSON_OBJECT}");
    add_rule("JSON_VALUE", b"{JSON_ARRAY}");
    add_rule("JSON_VALUE", b"\"{STRING_CONTENT}\"");
    add_rule("JSON_VALUE", b"{NUMBER}");
    add_rule("JSON_VALUE", b"true");
    add_rule("JSON_VALUE", b"false");
    add_rule("JSON_VALUE", b"null");

    // String content without quotes
    add_rule("STRING_CONTENT", b"{CHAR}");
    add_rule("STRING_CONTENT", b"{CHAR}{STRING_CONTENT}");

    // Common Parameters
    add_rule("PARAMS", b"{INITIALIZE_PARAMS}");
    add_rule("PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    add_rule("PARAMS", b"{WORKSPACE_PARAMS}");
    add_rule("PARAMS", b"null");

    // Basic message types
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"initialize\",\"params\":{INITIALIZE_PARAMS}\\}");
    add_rule(
        "REQUEST",
        b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"shutdown\",\"params\":null\\}",
    );
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/willSaveWaitUntil\",\"params\":{WILL_SAVE_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/completion\",\"params\":{COMPLETION_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"completionItem/resolve\",\"params\":{COMPLETION_ITEM}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/hover\",\"params\":{HOVER_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/signatureHelp\",\"params\":{SIGNATURE_HELP_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/definition\",\"params\":{DEFINITION_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentSymbol\",\"params\":{DOCUMENT_SYMBOL_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/codeAction\",\"params\":{CODE_ACTION_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/formatting\",\"params\":{FORMATTING_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/rename\",\"params\":{RENAME_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/rangeFormatting\",\"params\":{RANGE_FORMATTING_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/references\",\"params\":{REFERENCE_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/implementation\",\"params\":{IMPLEMENTATION_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/symbol\",\"params\":{WORKSPACE_SYMBOL_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentHighlight\",\"params\":{DOCUMENT_HIGHLIGHT_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/typeDefinition\",\"params\":{TYPE_DEFINITION_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/declaration\",\"params\":{DECLARATION_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/foldingRange\",\"params\":{FOLDING_RANGE_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/selectionRange\",\"params\":{SELECTION_RANGE_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/linkedEditingRange\",\"params\":{LINKED_EDITING_RANGE_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/prepareRename\",\"params\":{PREPARE_RENAME_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/codeLens\",\"params\":{CODE_LENS_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentColor\",\"params\":{DOCUMENT_COLOR_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/colorPresentation\",\"params\":{COLOR_PRESENTATION_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/prepareCallHierarchy\",\"params\":{PREPARE_CALL_HIERARCHY_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/semanticTokens/full\",\"params\":{SEMANTIC_TOKENS_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/moniker\",\"params\":{MONIKER_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/inlineValue\",\"params\":{INLINE_VALUE_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/willCreateFiles\",\"params\":{CREATE_FILES_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/executeCommand\",\"params\":{EXECUTE_COMMAND_PARAMS}\\}");
    // Additional requests that were missing
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/onTypeFormatting\",\"params\":{ON_TYPE_FORMATTING_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentLink\",\"params\":{DOCUMENT_LINK_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"documentLink/resolve\",\"params\":{DOCUMENT_LINK}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"codeLens/resolve\",\"params\":{CODE_LENS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/willRenameFiles\",\"params\":{RENAME_FILES_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/willDeleteFiles\",\"params\":{DELETE_FILES_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"callHierarchy/incomingCalls\",\"params\":{CALL_HIERARCHY_INCOMING_CALLS_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"callHierarchy/outgoingCalls\",\"params\":{CALL_HIERARCHY_OUTGOING_CALLS_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/semanticTokens/range\",\"params\":{SEMANTIC_TOKENS_RANGE_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/semanticTokens/full/delta\",\"params\":{SEMANTIC_TOKENS_DELTA_PARAMS}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"inlineCompletion/resolve\",\"params\":{INLINE_COMPLETION_ITEM}\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/inlineCompletion\",\"params\":{INLINE_COMPLETION_PARAMS}\\}");

    add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}\\}",
    );
    add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"exit\",\"params\":null\\}",
    );
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{DID_OPEN_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didChange\",\"params\":{DID_CHANGE_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didSave\",\"params\":{DID_SAVE_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didClose\",\"params\":{TEXT_DOCUMENT_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/willSave\",\"params\":{WILL_SAVE_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didChangeConfiguration\",\"params\":{WORKSPACE_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didChangeWatchedFiles\",\"params\":{DID_CHANGE_WATCHED_FILES_PARAMS}\\}");
    add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"/cancelRequest\",\"params\":{CANCEL_PARAMS}\\}",
    );
    // Additional notifications that were missing
    add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"$/progress\",\"params\":{PROGRESS_PARAMS}\\}",
    );
    add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"$/setTrace\",\"params\":{SET_TRACE_PARAMS}\\}",
    );
    add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"$/logTrace\",\"params\":{LOG_TRACE_PARAMS}\\}",
    );
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didCreateFiles\",\"params\":{CREATE_FILES_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didRenameFiles\",\"params\":{RENAME_FILES_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didDeleteFiles\",\"params\":{DELETE_FILES_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didOpen\",\"params\":{NOTEBOOK_DOCUMENT_DID_OPEN_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didChange\",\"params\":{NOTEBOOK_DOCUMENT_DID_CHANGE_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didSave\",\"params\":{NOTEBOOK_DOCUMENT_DID_SAVE_PARAMS}\\}");
    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didClose\",\"params\":{NOTEBOOK_DOCUMENT_DID_CLOSE_PARAMS}\\}");

    // TextDocumentItem for didOpen
    add_rule("TEXT_DOCUMENT_ITEM", b"\\{\"uri\":\"{URI}\",\"languageId\":\"{LANGUAGE_ID}\",\"version\":{NUMBER},\"text\":\"{TEXT}\"\\}");
    add_rule("URI", b"file:///{STRING_CONTENT}");
    add_rule("LANGUAGE_ID", b"rust");
    add_rule("LANGUAGE_ID", b"python");
    add_rule("LANGUAGE_ID", b"javascript");
    add_rule("LANGUAGE_ID", b"typescript");
    add_rule("LANGUAGE_ID", b"c");
    add_rule("LANGUAGE_ID", b"cpp");
    add_rule("TEXT", b"{STRING_CONTENT}");

    // Initialize params
    add_rule("INITIALIZE_PARAMS", b"\\{\"processId\":{NUMBER},\"rootUri\":\"file:///path/to/workspace\",\"capabilities\":{CLIENT_CAPABILITIES}\\}");
    add_rule(
        "CLIENT_CAPABILITIES",
        b"\\{\"workspace\":{WORKSPACE_CAPABILITY},\"textDocument\":{TEXT_DOCUMENT_CAPABILITY},\"window\":{WINDOW_CAPABILITY},\"general\":{GENERAL_CAPABILITY}\\}",
    );
    add_rule(
        "WORKSPACE_CAPABILITY",
        b"\\{\"applyEdit\":true,\"workspaceEdit\":{WORKSPACE_EDIT_CAPABILITY},\"didChangeConfiguration\":{DYNAMIC_REGISTRATION},\"didChangeWatchedFiles\":{DYNAMIC_REGISTRATION},\"symbol\":{SYMBOL_CAPABILITY},\"executeCommand\":{DYNAMIC_REGISTRATION},\"workspaceFolders\":true,\"configuration\":true\\}",
    );
    add_rule(
        "WORKSPACE_EDIT_CAPABILITY",
        b"\\{\"documentChanges\":true\\}",
    );
    add_rule(
        "TEXT_DOCUMENT_CAPABILITY",
        b"\\{\"synchronization\":{SYNC_CAPABILITY},\"completion\":{COMPLETION_CAPABILITY},\"hover\":{HOVER_CAPABILITY},\"signatureHelp\":{SIGNATURE_HELP_CAPABILITY},\"declaration\":{DECLARATION_CAPABILITY},\"definition\":{DEFINITION_CAPABILITY},\"typeDefinition\":{TYPE_DEFINITION_CAPABILITY},\"implementation\":{IMPLEMENTATION_CAPABILITY},\"references\":{REFERENCES_CAPABILITY},\"documentHighlight\":{DOCUMENT_HIGHLIGHT_CAPABILITY},\"documentSymbol\":{DOCUMENT_SYMBOL_CAPABILITY},\"codeAction\":{CODE_ACTION_CAPABILITY},\"codeLens\":{CODE_LENS_CAPABILITY},\"formatting\":{FORMATTING_CAPABILITY},\"rangeFormatting\":{RANGE_FORMATTING_CAPABILITY},\"onTypeFormatting\":{ON_TYPE_FORMATTING_CAPABILITY},\"rename\":{RENAME_CAPABILITY},\"publishDiagnostics\":{PUBLISH_DIAGNOSTICS_CAPABILITY},\"foldingRange\":{FOLDING_RANGE_CAPABILITY},\"selectionRange\":{SELECTION_RANGE_CAPABILITY},\"linkedEditingRange\":{LINKED_EDITING_RANGE_CAPABILITY},\"callHierarchy\":{CALL_HIERARCHY_CAPABILITY},\"semanticTokens\":{SEMANTIC_TOKENS_CAPABILITY},\"moniker\":{MONIKER_CAPABILITY},\"inlayHint\":{INLAY_HINT_CAPABILITY}\\}",
    );
    add_rule("SYNC_CAPABILITY", b"\\{\"dynamicRegistration\":true,\"willSave\":true,\"willSaveWaitUntil\":true,\"didSave\":true\\}");
    add_rule(
        "COMPLETION_CAPABILITY",
        b"\\{\"dynamicRegistration\":true,\"completionItem\":{COMPLETION_ITEM_CAPABILITY}\\}",
    );
    add_rule(
        "COMPLETION_ITEM_CAPABILITY",
        b"\\{\"snippetSupport\":true,\"commitCharactersSupport\":true\\}",
    );

    // Text document params
    add_rule(
        "TEXT_DOCUMENT_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER}\\}",
    );
    add_rule(
        "VERSIONED_TEXT_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":\"{URI}\",\"version\":{NUMBER}\\}",
    );
    add_rule(
        "OPTIONAL_VERSIONED_TEXT_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":\"{URI}\",\"version\":{NUMBER}\\}",
    );
    add_rule(
        "OPTIONAL_VERSIONED_TEXT_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":\"{URI}\",\"version\":null\\}",
    );
    add_rule(
        "TEXT_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":\"{URI}\",\"version\":{NUMBER}\\}",
    );

    // Position based params
    add_rule(
        "POSITION",
        b"\\{\"line\":{NUMBER},\"character\":{NUMBER}\\}",
    );
    add_rule(
        "TEXT_DOCUMENT_POSITION_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION}\\}",
    );

    // Range
    add_rule("RANGE", b"\\{\"start\":{POSITION},\"end\":{POSITION}\\}");

    // Completion params
    add_rule("COMPLETION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    add_rule("COMPLETION_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"context\":{COMPLETION_CONTEXT}\\}");
    add_rule(
        "COMPLETION_CONTEXT",
        b"\\{\"triggerKind\":{NUMBER},\"triggerCharacter\":\"{CHAR}\"\\}",
    );
    add_rule("COMPLETION_ITEM", b"\\{\"label\":\"{STRING_CONTENT}\",\"kind\":{NUMBER},\"detail\":\"{STRING_CONTENT}\",\"documentation\":\"{STRING_CONTENT}\",\"deprecated\":false,\"preselect\":false,\"sortText\":\"{STRING_CONTENT}\",\"filterText\":\"{STRING_CONTENT}\",\"insertText\":\"{STRING_CONTENT}\",\"insertTextFormat\":{NUMBER},\"textEdit\":{TEXT_EDIT},\"additionalTextEdits\":[{TEXT_EDIT}],\"commitCharacters\":[\"{CHAR}\"],\"command\":{COMMAND},\"data\":{JSON_VALUE}\\}");
    add_rule(
        "TEXT_EDIT",
        b"\\{\"range\":{RANGE},\"newText\":\"{STRING_CONTENT}\"\\}",
    );

    // Hover params
    add_rule("HOVER_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    add_rule(
        "HOVER",
        b"\\{\"contents\":{MARKUP_CONTENT},\"range\":{RANGE}\\}",
    );
    add_rule(
        "MARKUP_CONTENT",
        b"\\{\"kind\":\"markdown\",\"value\":\"{STRING_CONTENT}\"\\}",
    );
    add_rule(
        "MARKUP_CONTENT",
        b"\\{\"kind\":\"plaintext\",\"value\":\"{STRING_CONTENT}\"\\}",
    );

    // Signature help params
    add_rule("SIGNATURE_HELP_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    add_rule("SIGNATURE_HELP_CONTEXT", b"\\{\"isRetrigger\":true,\"triggerCharacter\":\"{CHAR}\",\"activeSignatureHelp\":{SIGNATURE_HELP}\\}");
    add_rule("SIGNATURE_HELP", b"\\{\"signatures\":[{SIGNATURE_INFORMATION}],\"activeSignature\":{NUMBER},\"activeParameter\":{NUMBER}\\}");
    add_rule("SIGNATURE_INFORMATION", b"\\{\"label\":\"{STRING_CONTENT}\",\"documentation\":\"{STRING_CONTENT}\",\"parameters\":[{PARAMETER_INFORMATION}]\\}");
    add_rule(
        "PARAMETER_INFORMATION",
        b"\\{\"label\":\"{STRING_CONTENT}\",\"documentation\":\"{STRING_CONTENT}\"\\}",
    );

    // Definition params
    add_rule("DEFINITION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    add_rule("LOCATION", b"\\{\"uri\":\"{URI}\",\"range\":{RANGE}\\}");
    add_rule("LOCATION_LINK", b"\\{\"originSelectionRange\":{RANGE},\"targetUri\":\"{URI}\",\"targetRange\":{RANGE},\"targetSelectionRange\":{RANGE}\\}");

    // References params
    add_rule("REFERENCE_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"context\":{REFERENCE_CONTEXT}\\}");
    add_rule("REFERENCE_CONTEXT", b"\\{\"includeDeclaration\":true\\}");

    // Document symbol params
    add_rule("DOCUMENT_SYMBOL_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Code action params
    add_rule("CODE_ACTION_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE},\"context\":{CODE_ACTION_CONTEXT}\\}");
    add_rule(
        "CODE_ACTION_CONTEXT",
        b"\\{\"diagnostics\":[{DIAGNOSTIC}]\\}",
    );
    add_rule("DIAGNOSTIC", b"\\{\"range\":{RANGE},\"severity\":{NUMBER},\"code\":{NUMBER},\"source\":\"{STRING_CONTENT}\",\"message\":\"{STRING_CONTENT}\",\"tags\":[{NUMBER}],\"relatedInformation\":[{DIAGNOSTIC_RELATED_INFORMATION}]\\}");
    add_rule(
        "DIAGNOSTIC_RELATED_INFORMATION",
        b"\\{\"location\":{LOCATION},\"message\":\"{STRING_CONTENT}\"\\}",
    );

    // Formatting params
    add_rule(
        "FORMATTING_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"options\":{FORMATTING_OPTIONS}\\}",
    );
    add_rule(
        "FORMATTING_OPTIONS",
        b"\\{\"tabSize\":{NUMBER},\"insertSpaces\":true\\}",
    );

    // Range formatting params
    add_rule("RANGE_FORMATTING_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE},\"options\":{FORMATTING_OPTIONS}\\}");

    // Rename params
    add_rule("RENAME_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"newName\":\"{STRING_CONTENT}\"\\}");
    add_rule("PREPARE_RENAME_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Implementation params
    add_rule("IMPLEMENTATION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Type definition params
    add_rule("TYPE_DEFINITION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Declaration params
    add_rule("DECLARATION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Document highlight params
    add_rule(
        "DOCUMENT_HIGHLIGHT_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );

    // Folding range params
    add_rule("FOLDING_RANGE_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Selection range params
    add_rule(
        "SELECTION_RANGE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"positions\":[{POSITION}]\\}",
    );

    // Linked editing range params
    add_rule(
        "LINKED_EDITING_RANGE_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );

    // Code lens params
    add_rule("CODE_LENS_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Document color params
    add_rule("DOCUMENT_COLOR_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    add_rule(
        "COLOR_PRESENTATION_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"color\":{COLOR},\"range\":{RANGE}\\}",
    );
    add_rule(
        "COLOR",
        b"\\{\"red\":{NUMBER},\"green\":{NUMBER},\"blue\":{NUMBER},\"alpha\":{NUMBER}\\}",
    );

    // Call hierarchy params
    add_rule(
        "PREPARE_CALL_HIERARCHY_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );

    // Semantic tokens params
    add_rule("SEMANTIC_TOKENS_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Moniker params
    add_rule("MONIKER_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    add_rule("MONIKER", b"\\{\"scheme\":\"{STRING_CONTENT}\",\"identifier\":\"{STRING_CONTENT}\",\"unique\":{NUMBER},\"kind\":{NUMBER}\\}");

    // Inline value params
    add_rule(
        "INLINE_VALUE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE}\\}",
    );

    // Create files params
    add_rule("CREATE_FILES_PARAMS", b"\\{\"files\":[{FILE_CREATE}]\\}");
    add_rule("FILE_CREATE", b"\\{\"uri\":\"{URI}\"\\}");

    // Execute command params
    add_rule(
        "EXECUTE_COMMAND_PARAMS",
        b"\\{\"command\":\"{STRING_CONTENT}\",\"arguments\":[{JSON_VALUE}]\\}",
    );

    for i in 1..=26 {
        add_rule("SYMBOL_KIND", format!("{}", i).as_bytes());
    }

    for i in 1..=25 {
        add_rule("COMPLETION_ITEM_KIND", format!("{}", i).as_bytes());
    }

    for i in 0..=2 {
        add_rule("TEXT_DOCUMENT_SYNC_KIND", format!("{}", i).as_bytes());
    }

    for i in 1..=4 {
        add_rule("DIAGNOSTIC_SEVERITY", format!("{}", i).as_bytes());
    }

    for i in 1..=2 {
        add_rule("INSERT_TEXT_FORMAT", format!("{}", i).as_bytes());
    }

    for i in 1..=3 {
        add_rule("DOCUMENT_HIGHLIGHT_KIND", format!("{}", i).as_bytes());
    }

    add_rule("CODE_ACTION_KIND", b"\"quickfix\"");
    add_rule("CODE_ACTION_KIND", b"\"refactor\"");
    add_rule("CODE_ACTION_KIND", b"\"refactor.extract\"");
    add_rule("CODE_ACTION_KIND", b"\"refactor.inline\"");
    add_rule("CODE_ACTION_KIND", b"\"refactor.rewrite\"");
    add_rule("CODE_ACTION_KIND", b"\"source\"");
    add_rule("CODE_ACTION_KIND", b"\"source.organizeImports\"");
    add_rule("CODE_ACTION_KIND", b"\"source.fixAll\"");

    add_rule("MARKUP_KIND", b"\"plaintext\"");
    add_rule("MARKUP_KIND", b"\"markdown\"");

    // Registration options
    add_rule(
        "TEXT_DOCUMENT_REGISTRATION_OPTIONS",
        b"\\{\"documentSelector\":[{DOCUMENT_FILTER}]\\}",
    );
    add_rule(
        "DOCUMENT_FILTER",
        b"\\{\"language\":\"{LANGUAGE_ID}\",\"scheme\":\"{URI_SCHEME}\",\"pattern\":\"**/*.{FILE_EXT}\"\\}",
    );
    add_rule("URI_SCHEME", b"file");
    add_rule("URI_SCHEME", b"untitled");
    add_rule("URI_SCHEME", b"git");

    add_rule(
        "STATIC_REGISTRATION_OPTIONS",
        b"\\{\"id\":\"{STRING_CONTENT}\"\\}",
    );

    add_rule(
        "WORK_DONE_PROGRESS_OPTIONS",
        b"\\{\"workDoneProgress\":true\\}",
    );
    add_rule(
        "WORK_DONE_PROGRESS_OPTIONS",
        b"\\{\"workDoneProgress\":false\\}",
    );

    add_rule(
        "CODE_ACTION_REGISTRATION_OPTIONS",
        b"\\{\"documentSelector\":[{DOCUMENT_FILTER}],\"codeActionKinds\":[{CODE_ACTION_KIND}]\\}",
    );

    add_rule(
        "COMPLETION_REGISTRATION_OPTIONS",
        b"\\{\"documentSelector\":[{DOCUMENT_FILTER}],\"triggerCharacters\":[\".\",[\":\"],[\"/\"]],\"allCommitCharacters\":[\".\",[\":\"],[\"/\"]],\"resolveProvider\":true\\}",
    );

    add_rule(
        "SIGNATURE_HELP_REGISTRATION_OPTIONS",
        b"\\{\"documentSelector\":[{DOCUMENT_FILTER}],\"triggerCharacters\":[\".\",[\":\"],[\"/\"]],\"retriggerCharacters\":[\".\",[\":\"],[\"/\"]]\\}",
    );

    // Notification params
    add_rule(
        "DID_OPEN_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_ITEM}\\}",
    );
    add_rule("DID_CHANGE_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"contentChanges\":[{TEXT_DOCUMENT_CONTENT_CHANGE_EVENT}]\\}");
    add_rule(
        "TEXT_DOCUMENT_CONTENT_CHANGE_EVENT",
        b"\\{\"text\":\"{TEXT}\"\\}",
    );
    add_rule(
        "DID_SAVE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"text\":\"{TEXT}\"\\}",
    );
    add_rule(
        "WILL_SAVE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"reason\":{NUMBER}\\}",
    );

    // Workspace params
    add_rule("WORKSPACE_PARAMS", b"\\{\"settings\":{JSON_OBJECT}\\}");

    // Watched files params
    add_rule(
        "DID_CHANGE_WATCHED_FILES_PARAMS",
        b"\\{\"changes\":[{FILE_EVENT}]\\}",
    );
    add_rule("FILE_EVENT", b"\\{\"uri\":\"{URI}\",\"type\":{NUMBER}\\}");

    // Cancel params
    add_rule("CANCEL_PARAMS", b"\\{\"id\":{NUMBER}\\}");

    // Workspace symbol params
    add_rule(
        "WORKSPACE_SYMBOL_PARAMS",
        b"\\{\"query\":\"{STRING_CONTENT}\"\\}",
    );

    // OnTypeFormatting params
    add_rule("ON_TYPE_FORMATTING_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"ch\":\"{CHAR}\",\"options\":{FORMATTING_OPTIONS}\\}");

    // DocumentLink params
    add_rule("DOCUMENT_LINK_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    add_rule("DOCUMENT_LINK", b"\\{\"range\":{RANGE},\"target\":\"{URI}\",\"tooltip\":\"{STRING_CONTENT}\",\"data\":{JSON_VALUE}\\}");

    // CodeLens item
    add_rule(
        "CODE_LENS",
        b"\\{\"range\":{RANGE},\"command\":{COMMAND},\"data\":{JSON_VALUE}\\}",
    );
    add_rule("COMMAND", b"\\{\"title\":\"{STRING_CONTENT}\",\"command\":\"{STRING_CONTENT}\",\"arguments\":[{JSON_VALUE}]\\}");

    // File operations params
    add_rule("RENAME_FILES_PARAMS", b"\\{\"files\":[{FILE_RENAME}]\\}");
    add_rule(
        "FILE_RENAME",
        b"\\{\"oldUri\":\"{URI}\",\"newUri\":\"{URI}\"\\}",
    );
    add_rule("DELETE_FILES_PARAMS", b"\\{\"files\":[{FILE_DELETE}]\\}");
    add_rule("FILE_DELETE", b"\\{\"uri\":\"{URI}\"\\}");

    // Call hierarchy item
    add_rule("CALL_HIERARCHY_ITEM", b"\\{\"name\":\"{STRING_CONTENT}\",\"kind\":{NUMBER},\"uri\":\"{URI}\",\"range\":{RANGE},\"selectionRange\":{RANGE},\"data\":{JSON_VALUE}\\}");
    add_rule(
        "CALL_HIERARCHY_INCOMING_CALLS_PARAMS",
        b"\\{\"item\":{CALL_HIERARCHY_ITEM}\\}",
    );
    add_rule(
        "CALL_HIERARCHY_OUTGOING_CALLS_PARAMS",
        b"\\{\"item\":{CALL_HIERARCHY_ITEM}\\}",
    );
    add_rule(
        "CALL_HIERARCHY_INCOMING_CALL",
        b"\\{\"from\":{CALL_HIERARCHY_ITEM},\"fromRanges\":[{RANGE}]\\}",
    );
    add_rule(
        "CALL_HIERARCHY_OUTGOING_CALL",
        b"\\{\"to\":{CALL_HIERARCHY_ITEM},\"fromRanges\":[{RANGE}]\\}",
    );

    // Semantic tokens params
    add_rule(
        "SEMANTIC_TOKENS_RANGE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE}\\}",
    );
    add_rule("SEMANTIC_TOKENS_DELTA_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"previousResultId\":\"{STRING_CONTENT}\"\\}");
    add_rule(
        "SEMANTIC_TOKENS",
        b"\\{\"resultId\":\"{STRING_CONTENT}\",\"data\":[{NUMBER}]\\}",
    );

    // Inline completion
    add_rule(
        "INLINE_COMPLETION_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );
    add_rule(
        "INLINE_COMPLETION_ITEM",
        b"\\{\"insertText\":\"{STRING_CONTENT}\",\"range\":{RANGE}\\}",
    );

    // Progress notification
    add_rule(
        "PROGRESS_PARAMS",
        b"\\{\"token\":{JSON_VALUE},\"value\":{JSON_OBJECT}\\}",
    );

    // Trace notification
    add_rule("SET_TRACE_PARAMS", b"\\{\"value\":\"{STRING_CONTENT}\"\\}");
    add_rule(
        "LOG_TRACE_PARAMS",
        b"\\{\"message\":\"{STRING_CONTENT}\",\"verbose\":\"{STRING_CONTENT}\"\\}",
    );

    // Workspace methods
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/workspaceFolders\",\"params\":null\\}");
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/configuration\",\"params\":{CONFIGURATION_PARAMS}\\}");

    add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didChangeWorkspaceFolders\",\"params\":{WORKSPACE_FOLDERS_CHANGE_EVENT}\\}");

    // Text document methods
    add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/inlayHint\",\"params\":{INLAY_HINT_PARAMS}\\}");

    // Parameter types for new methods
    add_rule(
        "CONFIGURATION_PARAMS",
        b"\\{\"items\":[{CONFIGURATION_ITEM}]\\}",
    );
    add_rule(
        "CONFIGURATION_ITEM",
        b"\\{\"scopeUri\":\"{URI}\",\"section\":\"{STRING_CONTENT}\"\\}",
    );

    add_rule(
        "WORKSPACE_FOLDERS_CHANGE_EVENT",
        b"\\{\"added\":[{WORKSPACE_FOLDER}],\"removed\":[{WORKSPACE_FOLDER}]\\}",
    );
    add_rule(
        "WORKSPACE_FOLDER",
        b"\\{\"uri\":\"{URI}\",\"name\":\"{STRING_CONTENT}\"\\}",
    );

    add_rule("INLAY_HINT_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Notebook document params
    add_rule("NOTEBOOK_DOCUMENT_DID_OPEN_PARAMS", b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT},\"cellTextDocuments\":[{TEXT_DOCUMENT_ITEM}]\\}");
    add_rule("NOTEBOOK_DOCUMENT_DID_CHANGE_PARAMS", b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT_IDENTIFIER},\"change\":{NOTEBOOK_DOCUMENT_CHANGE_EVENT}\\}");
    add_rule(
        "NOTEBOOK_DOCUMENT_DID_SAVE_PARAMS",
        b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT_IDENTIFIER}\\}",
    );
    add_rule("NOTEBOOK_DOCUMENT_DID_CLOSE_PARAMS", b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT_IDENTIFIER},\"cellTextDocuments\":[{TEXT_DOCUMENT_IDENTIFIER}]\\}");

    // Notebook document
    add_rule("NOTEBOOK_DOCUMENT", b"\\{\"uri\":\"{URI}\",\"notebookType\":\"{STRING_CONTENT}\",\"version\":{NUMBER},\"cells\":[{NOTEBOOK_CELL}]\\}");
    add_rule(
        "NOTEBOOK_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":\"{URI}\",\"version\":{NUMBER}\\}",
    );
    add_rule(
        "NOTEBOOK_DOCUMENT_CHANGE_EVENT",
        b"\\{\"cells\":{CELLS_CHANGE_EVENT}\\}",
    );
    add_rule("CELLS_CHANGE_EVENT", b"\\{\"structure\":{CELLS_STRUCTURE_CHANGE_EVENT},\"data\":[{NOTEBOOK_CELL}],\"textContent\":[{CELL_TEXT_CONTENT_CHANGE_EVENT}]\\}");
    add_rule("CELLS_STRUCTURE_CHANGE_EVENT", b"\\{\"array\":{ARRAY_CHANGE_EVENT},\"didOpen\":[{TEXT_DOCUMENT_ITEM}],\"didClose\":[{TEXT_DOCUMENT_IDENTIFIER}]\\}");
    add_rule(
        "ARRAY_CHANGE_EVENT",
        b"\\{\"start\":{NUMBER},\"deleteCount\":{NUMBER},\"cells\":[{NOTEBOOK_CELL}]\\}",
    );
    add_rule("CELL_TEXT_CONTENT_CHANGE_EVENT", b"\\{\"document\":{TEXT_DOCUMENT_IDENTIFIER},\"changes\":[{TEXT_DOCUMENT_CONTENT_CHANGE_EVENT}]\\}");
    add_rule(
        "NOTEBOOK_CELL",
        b"\\{\"kind\":{NUMBER},\"document\":\"{URI}\"\\}",
    );

    rules
}
