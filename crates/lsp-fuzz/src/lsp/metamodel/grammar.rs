use libafl::nautilus::grammartec::context::Context;

pub fn get_grammar() -> Context {
    let mut ctx = Context::new();

    // Core message structure
    ctx.add_rule("START", b"{REQUEST}");
    ctx.add_rule("START", b"{NOTIFICATION}");

    // Numbers
    ctx.add_rule("NUMBER", b"{DIGIT}");
    ctx.add_rule("NUMBER", b"{DIGIT}{NUMBER}");
    for i in 0..=9 {
        ctx.add_rule("DIGIT", &[b'0' + i]);
    }

    // Strings
    ctx.add_rule("STRING", b"\"{CHAR}\"");
    ctx.add_rule("STRING", b"{CHAR}{STRING}");
    ctx.add_rule("CHAR", b"{DIGIT}");
    for letter in b'a'..=b'z' {
        ctx.add_rule("CHAR", &[letter]);
    }
    for letter in b'A'..=b'Z' {
        ctx.add_rule("CHAR", &[letter]);
    }
    ctx.add_rule("CHAR", b"_");
    ctx.add_rule("CHAR", b"-");
    ctx.add_rule("CHAR", b"/");
    ctx.add_rule("CHAR", b".");
    ctx.add_rule("CHAR", b",");
    ctx.add_rule("CHAR", b":");
    ctx.add_rule("CHAR", b"$");
    ctx.add_rule("CHAR", b"@");
    ctx.add_rule("CHAR", b"#");
    ctx.add_rule("CHAR", b"!");
    ctx.add_rule("CHAR", b"?");
    ctx.add_rule("CHAR", b"+");
    ctx.add_rule("CHAR", b"*");
    ctx.add_rule("CHAR", b"&");
    ctx.add_rule("CHAR", b"%");
    ctx.add_rule("CHAR", b"=");

    // JSON Object
    ctx.add_rule("JSON_OBJECT", b"\\{\\}");
    ctx.add_rule("JSON_OBJECT", b"\\{{JSON_MEMBERS}\\}");
    ctx.add_rule("JSON_MEMBERS", b"{JSON_MEMBER}");
    ctx.add_rule("JSON_MEMBERS", b"{JSON_MEMBER},{JSON_MEMBERS}");
    ctx.add_rule("JSON_MEMBER", b"\"{STRING_CONTENT}\":{JSON_VALUE}");

    // JSON Array
    ctx.add_rule("JSON_ARRAY", b"\\[\\]");
    ctx.add_rule("JSON_ARRAY", b"\\[{JSON_ELEMENTS}\\]");
    ctx.add_rule("JSON_ELEMENTS", b"{JSON_VALUE}");
    ctx.add_rule("JSON_ELEMENTS", b"{JSON_VALUE},{JSON_ELEMENTS}");

    // JSON Value
    ctx.add_rule("JSON_VALUE", b"{JSON_OBJECT}");
    ctx.add_rule("JSON_VALUE", b"{JSON_ARRAY}");
    ctx.add_rule("JSON_VALUE", b"\"{STRING_CONTENT}\"");
    ctx.add_rule("JSON_VALUE", b"{NUMBER}");
    ctx.add_rule("JSON_VALUE", b"true");
    ctx.add_rule("JSON_VALUE", b"false");
    ctx.add_rule("JSON_VALUE", b"null");

    // String content without quotes
    ctx.add_rule("STRING_CONTENT", b"{CHAR}");
    ctx.add_rule("STRING_CONTENT", b"{CHAR}{STRING_CONTENT}");

    // Common Parameters
    ctx.add_rule("PARAMS", b"{INITIALIZE_PARAMS}");
    ctx.add_rule("PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    ctx.add_rule("PARAMS", b"{WORKSPACE_PARAMS}");
    ctx.add_rule("PARAMS", b"null");

    // Basic message types
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"initialize\",\"params\":{INITIALIZE_PARAMS}\\}");
    ctx.add_rule(
        "REQUEST",
        b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"shutdown\",\"params\":null\\}",
    );
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/willSaveWaitUntil\",\"params\":{WILL_SAVE_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/completion\",\"params\":{COMPLETION_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"completionItem/resolve\",\"params\":{COMPLETION_ITEM}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/hover\",\"params\":{HOVER_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/signatureHelp\",\"params\":{SIGNATURE_HELP_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/definition\",\"params\":{DEFINITION_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentSymbol\",\"params\":{DOCUMENT_SYMBOL_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/codeAction\",\"params\":{CODE_ACTION_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/formatting\",\"params\":{FORMATTING_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/rename\",\"params\":{RENAME_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/rangeFormatting\",\"params\":{RANGE_FORMATTING_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/references\",\"params\":{REFERENCE_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/implementation\",\"params\":{IMPLEMENTATION_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/symbol\",\"params\":{WORKSPACE_SYMBOL_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentHighlight\",\"params\":{DOCUMENT_HIGHLIGHT_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/typeDefinition\",\"params\":{TYPE_DEFINITION_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/declaration\",\"params\":{DECLARATION_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/foldingRange\",\"params\":{FOLDING_RANGE_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/selectionRange\",\"params\":{SELECTION_RANGE_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/linkedEditingRange\",\"params\":{LINKED_EDITING_RANGE_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/prepareRename\",\"params\":{PREPARE_RENAME_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/codeLens\",\"params\":{CODE_LENS_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentColor\",\"params\":{DOCUMENT_COLOR_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/colorPresentation\",\"params\":{COLOR_PRESENTATION_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/prepareCallHierarchy\",\"params\":{PREPARE_CALL_HIERARCHY_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/semanticTokens/full\",\"params\":{SEMANTIC_TOKENS_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/moniker\",\"params\":{MONIKER_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/inlineValue\",\"params\":{INLINE_VALUE_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/willCreateFiles\",\"params\":{CREATE_FILES_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/executeCommand\",\"params\":{EXECUTE_COMMAND_PARAMS}\\}");
    // Additional requests that were missing
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/onTypeFormatting\",\"params\":{ON_TYPE_FORMATTING_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/documentLink\",\"params\":{DOCUMENT_LINK_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"documentLink/resolve\",\"params\":{DOCUMENT_LINK}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"codeLens/resolve\",\"params\":{CODE_LENS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/willRenameFiles\",\"params\":{RENAME_FILES_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"workspace/willDeleteFiles\",\"params\":{DELETE_FILES_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"callHierarchy/incomingCalls\",\"params\":{CALL_HIERARCHY_INCOMING_CALLS_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"callHierarchy/outgoingCalls\",\"params\":{CALL_HIERARCHY_OUTGOING_CALLS_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/semanticTokens/range\",\"params\":{SEMANTIC_TOKENS_RANGE_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/semanticTokens/full/delta\",\"params\":{SEMANTIC_TOKENS_DELTA_PARAMS}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"inlineCompletion/resolve\",\"params\":{INLINE_COMPLETION_ITEM}\\}");
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":\"textDocument/inlineCompletion\",\"params\":{INLINE_COMPLETION_PARAMS}\\}");

    ctx.add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}\\}",
    );
    ctx.add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"exit\",\"params\":null\\}",
    );
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{DID_OPEN_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didChange\",\"params\":{DID_CHANGE_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didSave\",\"params\":{DID_SAVE_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didClose\",\"params\":{TEXT_DOCUMENT_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/willSave\",\"params\":{WILL_SAVE_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didChangeConfiguration\",\"params\":{WORKSPACE_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didChangeWatchedFiles\",\"params\":{DID_CHANGE_WATCHED_FILES_PARAMS}\\}");
    ctx.add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"/cancelRequest\",\"params\":{CANCEL_PARAMS}\\}",
    );
    // Additional notifications that were missing
    ctx.add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"$/progress\",\"params\":{PROGRESS_PARAMS}\\}",
    );
    ctx.add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"$/setTrace\",\"params\":{SET_TRACE_PARAMS}\\}",
    );
    ctx.add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":\"$/logTrace\",\"params\":{LOG_TRACE_PARAMS}\\}",
    );
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didCreateFiles\",\"params\":{CREATE_FILES_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didRenameFiles\",\"params\":{RENAME_FILES_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"workspace/didDeleteFiles\",\"params\":{DELETE_FILES_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didOpen\",\"params\":{NOTEBOOK_DOCUMENT_DID_OPEN_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didChange\",\"params\":{NOTEBOOK_DOCUMENT_DID_CHANGE_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didSave\",\"params\":{NOTEBOOK_DOCUMENT_DID_SAVE_PARAMS}\\}");
    ctx.add_rule("NOTIFICATION", b"\\{\"jsonrpc\":\"2.0\",\"method\":\"notebookDocument/didClose\",\"params\":{NOTEBOOK_DOCUMENT_DID_CLOSE_PARAMS}\\}");

    // TextDocumentItem for didOpen
    ctx.add_rule("TEXT_DOCUMENT_ITEM", b"\\{\"uri\":\"{URI}\",\"languageId\":\"{LANGUAGE_ID}\",\"version\":{NUMBER},\"text\":\"{TEXT}\"\\}");
    ctx.add_rule("URI", b"file:///path/to/file.{FILE_EXT}");
    ctx.add_rule("LANGUAGE_ID", b"rust");
    ctx.add_rule("LANGUAGE_ID", b"python");
    ctx.add_rule("LANGUAGE_ID", b"javascript");
    ctx.add_rule("LANGUAGE_ID", b"typescript");
    ctx.add_rule("LANGUAGE_ID", b"c");
    ctx.add_rule("LANGUAGE_ID", b"cpp");
    ctx.add_rule("FILE_EXT", b"rs");
    ctx.add_rule("FILE_EXT", b"py");
    ctx.add_rule("FILE_EXT", b"js");
    ctx.add_rule("FILE_EXT", b"ts");
    ctx.add_rule("FILE_EXT", b"c");
    ctx.add_rule("FILE_EXT", b"cpp");
    ctx.add_rule("TEXT", b"{STRING_CONTENT}");

    // Initialize params
    ctx.add_rule("INITIALIZE_PARAMS", b"\\{\"processId\":{NUMBER},\"rootUri\":\"file:///path/to/workspace\",\"capabilities\":{CLIENT_CAPABILITIES}\\}");
    ctx.add_rule(
        "CLIENT_CAPABILITIES",
        b"\\{\"workspace\":{WORKSPACE_CAPABILITY},\"textDocument\":{TEXT_DOCUMENT_CAPABILITY}\\}",
    );
    ctx.add_rule(
        "WORKSPACE_CAPABILITY",
        b"\\{\"applyEdit\":true,\"workspaceEdit\":{WORKSPACE_EDIT_CAPABILITY}\\}",
    );
    ctx.add_rule(
        "WORKSPACE_EDIT_CAPABILITY",
        b"\\{\"documentChanges\":true\\}",
    );
    ctx.add_rule(
        "TEXT_DOCUMENT_CAPABILITY",
        b"\\{\"synchronization\":{SYNC_CAPABILITY},\"completion\":{COMPLETION_CAPABILITY}\\}",
    );
    ctx.add_rule("SYNC_CAPABILITY", b"\\{\"dynamicRegistration\":true,\"willSave\":true,\"willSaveWaitUntil\":true,\"didSave\":true\\}");
    ctx.add_rule(
        "COMPLETION_CAPABILITY",
        b"\\{\"dynamicRegistration\":true,\"completionItem\":{COMPLETION_ITEM_CAPABILITY}\\}",
    );
    ctx.add_rule(
        "COMPLETION_ITEM_CAPABILITY",
        b"\\{\"snippetSupport\":true,\"commitCharactersSupport\":true\\}",
    );

    // Text document params
    ctx.add_rule(
        "TEXT_DOCUMENT_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER}\\}",
    );
    ctx.add_rule(
        "TEXT_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":\"{URI}\",\"version\":{NUMBER}\\}",
    );

    // Position based params
    ctx.add_rule(
        "POSITION",
        b"\\{\"line\":{NUMBER},\"character\":{NUMBER}\\}",
    );
    ctx.add_rule(
        "TEXT_DOCUMENT_POSITION_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION}\\}",
    );

    // Range
    ctx.add_rule("RANGE", b"\\{\"start\":{POSITION},\"end\":{POSITION}\\}");

    // Completion params
    ctx.add_rule("COMPLETION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    ctx.add_rule("COMPLETION_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"context\":{COMPLETION_CONTEXT}\\}");
    ctx.add_rule(
        "COMPLETION_CONTEXT",
        b"\\{\"triggerKind\":{NUMBER},\"triggerCharacter\":\"{CHAR}\"\\}",
    );
    ctx.add_rule("COMPLETION_ITEM", b"\\{\"label\":\"{STRING_CONTENT}\",\"kind\":{NUMBER},\"detail\":\"{STRING_CONTENT}\",\"documentation\":\"{STRING_CONTENT}\",\"deprecated\":false,\"preselect\":false,\"sortText\":\"{STRING_CONTENT}\",\"filterText\":\"{STRING_CONTENT}\",\"insertText\":\"{STRING_CONTENT}\",\"insertTextFormat\":{NUMBER},\"textEdit\":{TEXT_EDIT},\"additionalTextEdits\":[{TEXT_EDIT}],\"commitCharacters\":[\"{CHAR}\"],\"command\":{COMMAND},\"data\":{JSON_VALUE}\\}");
    ctx.add_rule(
        "TEXT_EDIT",
        b"\\{\"range\":{RANGE},\"newText\":\"{STRING_CONTENT}\"\\}",
    );

    // Hover params
    ctx.add_rule("HOVER_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    ctx.add_rule(
        "HOVER",
        b"\\{\"contents\":{MARKUP_CONTENT},\"range\":{RANGE}\\}",
    );
    ctx.add_rule(
        "MARKUP_CONTENT",
        b"\\{\"kind\":\"markdown\",\"value\":\"{STRING_CONTENT}\"\\}",
    );
    ctx.add_rule(
        "MARKUP_CONTENT",
        b"\\{\"kind\":\"plaintext\",\"value\":\"{STRING_CONTENT}\"\\}",
    );

    // Signature help params
    ctx.add_rule("SIGNATURE_HELP_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    ctx.add_rule("SIGNATURE_HELP_CONTEXT", b"\\{\"isRetrigger\":true,\"triggerCharacter\":\"{CHAR}\",\"activeSignatureHelp\":{SIGNATURE_HELP}\\}");
    ctx.add_rule("SIGNATURE_HELP", b"\\{\"signatures\":[{SIGNATURE_INFORMATION}],\"activeSignature\":{NUMBER},\"activeParameter\":{NUMBER}\\}");
    ctx.add_rule("SIGNATURE_INFORMATION", b"\\{\"label\":\"{STRING_CONTENT}\",\"documentation\":\"{STRING_CONTENT}\",\"parameters\":[{PARAMETER_INFORMATION}]\\}");
    ctx.add_rule(
        "PARAMETER_INFORMATION",
        b"\\{\"label\":\"{STRING_CONTENT}\",\"documentation\":\"{STRING_CONTENT}\"\\}",
    );

    // Definition params
    ctx.add_rule("DEFINITION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    ctx.add_rule("LOCATION", b"\\{\"uri\":\"{URI}\",\"range\":{RANGE}\\}");
    ctx.add_rule("LOCATION_LINK", b"\\{\"originSelectionRange\":{RANGE},\"targetUri\":\"{URI}\",\"targetRange\":{RANGE},\"targetSelectionRange\":{RANGE}\\}");

    // References params
    ctx.add_rule("REFERENCE_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"context\":{REFERENCE_CONTEXT}\\}");
    ctx.add_rule("REFERENCE_CONTEXT", b"\\{\"includeDeclaration\":true\\}");

    // Document symbol params
    ctx.add_rule("DOCUMENT_SYMBOL_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Code action params
    ctx.add_rule("CODE_ACTION_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE},\"context\":{CODE_ACTION_CONTEXT}\\}");
    ctx.add_rule(
        "CODE_ACTION_CONTEXT",
        b"\\{\"diagnostics\":[{DIAGNOSTIC}]\\}",
    );
    ctx.add_rule("DIAGNOSTIC", b"\\{\"range\":{RANGE},\"severity\":{NUMBER},\"code\":{NUMBER},\"source\":\"{STRING_CONTENT}\",\"message\":\"{STRING_CONTENT}\",\"tags\":[{NUMBER}],\"relatedInformation\":[{DIAGNOSTIC_RELATED_INFORMATION}]\\}");
    ctx.add_rule(
        "DIAGNOSTIC_RELATED_INFORMATION",
        b"\\{\"location\":{LOCATION},\"message\":\"{STRING_CONTENT}\"\\}",
    );

    // Formatting params
    ctx.add_rule(
        "FORMATTING_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"options\":{FORMATTING_OPTIONS}\\}",
    );
    ctx.add_rule(
        "FORMATTING_OPTIONS",
        b"\\{\"tabSize\":{NUMBER},\"insertSpaces\":true\\}",
    );

    // Range formatting params
    ctx.add_rule("RANGE_FORMATTING_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE},\"options\":{FORMATTING_OPTIONS}\\}");

    // Rename params
    ctx.add_rule("RENAME_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"newName\":\"{STRING_CONTENT}\"\\}");
    ctx.add_rule("PREPARE_RENAME_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Implementation params
    ctx.add_rule("IMPLEMENTATION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Type definition params
    ctx.add_rule("TYPE_DEFINITION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Declaration params
    ctx.add_rule("DECLARATION_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");

    // Document highlight params
    ctx.add_rule(
        "DOCUMENT_HIGHLIGHT_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );

    // Folding range params
    ctx.add_rule("FOLDING_RANGE_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Selection range params
    ctx.add_rule(
        "SELECTION_RANGE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"positions\":[{POSITION}]\\}",
    );

    // Linked editing range params
    ctx.add_rule(
        "LINKED_EDITING_RANGE_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );

    // Code lens params
    ctx.add_rule("CODE_LENS_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Document color params
    ctx.add_rule("DOCUMENT_COLOR_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    ctx.add_rule(
        "COLOR_PRESENTATION_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"color\":{COLOR},\"range\":{RANGE}\\}",
    );
    ctx.add_rule(
        "COLOR",
        b"\\{\"red\":{NUMBER},\"green\":{NUMBER},\"blue\":{NUMBER},\"alpha\":{NUMBER}\\}",
    );

    // Call hierarchy params
    ctx.add_rule(
        "PREPARE_CALL_HIERARCHY_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );

    // Semantic tokens params
    ctx.add_rule("SEMANTIC_TOKENS_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");

    // Moniker params
    ctx.add_rule("MONIKER_PARAMS", b"{TEXT_DOCUMENT_POSITION_PARAMS}");
    ctx.add_rule("MONIKER", b"\\{\"scheme\":\"{STRING_CONTENT}\",\"identifier\":\"{STRING_CONTENT}\",\"unique\":{NUMBER},\"kind\":{NUMBER}\\}");

    // Inline value params
    ctx.add_rule(
        "INLINE_VALUE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE}\\}",
    );

    // Create files params
    ctx.add_rule("CREATE_FILES_PARAMS", b"\\{\"files\":[{FILE_CREATE}]\\}");
    ctx.add_rule("FILE_CREATE", b"\\{\"uri\":\"{URI}\"\\}");

    // Execute command params
    ctx.add_rule(
        "EXECUTE_COMMAND_PARAMS",
        b"\\{\"command\":\"{STRING_CONTENT}\",\"arguments\":[{JSON_VALUE}]\\}",
    );

    // Notification params
    ctx.add_rule(
        "DID_OPEN_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_ITEM}\\}",
    );
    ctx.add_rule("DID_CHANGE_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"contentChanges\":[{TEXT_DOCUMENT_CONTENT_CHANGE_EVENT}]\\}");
    ctx.add_rule(
        "TEXT_DOCUMENT_CONTENT_CHANGE_EVENT",
        b"\\{\"text\":\"{TEXT}\"\\}",
    );
    ctx.add_rule(
        "DID_SAVE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"text\":\"{TEXT}\"\\}",
    );
    ctx.add_rule(
        "WILL_SAVE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"reason\":{NUMBER}\\}",
    );

    // Workspace params
    ctx.add_rule("WORKSPACE_PARAMS", b"\\{\"settings\":{JSON_OBJECT}\\}");

    // Watched files params
    ctx.add_rule(
        "DID_CHANGE_WATCHED_FILES_PARAMS",
        b"\\{\"changes\":[{FILE_EVENT}]\\}",
    );
    ctx.add_rule("FILE_EVENT", b"\\{\"uri\":\"{URI}\",\"type\":{NUMBER}\\}");

    // Cancel params
    ctx.add_rule("CANCEL_PARAMS", b"\\{\"id\":{NUMBER}\\}");

    // Workspace symbol params
    ctx.add_rule(
        "WORKSPACE_SYMBOL_PARAMS",
        b"\\{\"query\":\"{STRING_CONTENT}\"\\}",
    );

    // OnTypeFormatting params
    ctx.add_rule("ON_TYPE_FORMATTING_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"ch\":\"{CHAR}\",\"options\":{FORMATTING_OPTIONS}\\}");

    // DocumentLink params
    ctx.add_rule("DOCUMENT_LINK_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    ctx.add_rule("DOCUMENT_LINK", b"\\{\"range\":{RANGE},\"target\":\"{URI}\",\"tooltip\":\"{STRING_CONTENT}\",\"data\":{JSON_VALUE}\\}");

    // CodeLens item
    ctx.add_rule(
        "CODE_LENS",
        b"\\{\"range\":{RANGE},\"command\":{COMMAND},\"data\":{JSON_VALUE}\\}",
    );
    ctx.add_rule("COMMAND", b"\\{\"title\":\"{STRING_CONTENT}\",\"command\":\"{STRING_CONTENT}\",\"arguments\":[{JSON_VALUE}]\\}");

    // File operations params
    ctx.add_rule("RENAME_FILES_PARAMS", b"\\{\"files\":[{FILE_RENAME}]\\}");
    ctx.add_rule(
        "FILE_RENAME",
        b"\\{\"oldUri\":\"{URI}\",\"newUri\":\"{URI}\"\\}",
    );
    ctx.add_rule("DELETE_FILES_PARAMS", b"\\{\"files\":[{FILE_DELETE}]\\}");
    ctx.add_rule("FILE_DELETE", b"\\{\"uri\":\"{URI}\"\\}");

    // Call hierarchy item
    ctx.add_rule("CALL_HIERARCHY_ITEM", b"\\{\"name\":\"{STRING_CONTENT}\",\"kind\":{NUMBER},\"uri\":\"{URI}\",\"range\":{RANGE},\"selectionRange\":{RANGE},\"data\":{JSON_VALUE}\\}");
    ctx.add_rule(
        "CALL_HIERARCHY_INCOMING_CALLS_PARAMS",
        b"\\{\"item\":{CALL_HIERARCHY_ITEM}\\}",
    );
    ctx.add_rule(
        "CALL_HIERARCHY_OUTGOING_CALLS_PARAMS",
        b"\\{\"item\":{CALL_HIERARCHY_ITEM}\\}",
    );
    ctx.add_rule(
        "CALL_HIERARCHY_INCOMING_CALL",
        b"\\{\"from\":{CALL_HIERARCHY_ITEM},\"fromRanges\":[{RANGE}]\\}",
    );
    ctx.add_rule(
        "CALL_HIERARCHY_OUTGOING_CALL",
        b"\\{\"to\":{CALL_HIERARCHY_ITEM},\"fromRanges\":[{RANGE}]\\}",
    );

    // Semantic tokens params
    ctx.add_rule("SEMANTIC_TOKENS_PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    ctx.add_rule(
        "SEMANTIC_TOKENS_RANGE_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE}\\}",
    );
    ctx.add_rule("SEMANTIC_TOKENS_DELTA_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"previousResultId\":\"{STRING_CONTENT}\"\\}");
    ctx.add_rule(
        "SEMANTIC_TOKENS",
        b"\\{\"resultId\":\"{STRING_CONTENT}\",\"data\":[{NUMBER}]\\}",
    );

    // Inline completion
    ctx.add_rule(
        "INLINE_COMPLETION_PARAMS",
        b"{TEXT_DOCUMENT_POSITION_PARAMS}",
    );
    ctx.add_rule(
        "INLINE_COMPLETION_ITEM",
        b"\\{\"insertText\":\"{STRING_CONTENT}\",\"range\":{RANGE}\\}",
    );

    // Progress notification
    ctx.add_rule(
        "PROGRESS_PARAMS",
        b"\\{\"token\":{JSON_VALUE},\"value\":{JSON_OBJECT}\\}",
    );

    // Trace notification
    ctx.add_rule("SET_TRACE_PARAMS", b"\\{\"value\":\"{STRING_CONTENT}\"\\}");
    ctx.add_rule(
        "LOG_TRACE_PARAMS",
        b"\\{\"message\":\"{STRING_CONTENT}\",\"verbose\":\"{STRING_CONTENT}\"\\}",
    );

    // Notebook document params
    ctx.add_rule("NOTEBOOK_DOCUMENT_DID_OPEN_PARAMS", b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT},\"cellTextDocuments\":[{TEXT_DOCUMENT_ITEM}]\\}");
    ctx.add_rule("NOTEBOOK_DOCUMENT_DID_CHANGE_PARAMS", b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT_IDENTIFIER},\"change\":{NOTEBOOK_DOCUMENT_CHANGE_EVENT}\\}");
    ctx.add_rule(
        "NOTEBOOK_DOCUMENT_DID_SAVE_PARAMS",
        b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT_IDENTIFIER}\\}",
    );
    ctx.add_rule("NOTEBOOK_DOCUMENT_DID_CLOSE_PARAMS", b"\\{\"notebookDocument\":{NOTEBOOK_DOCUMENT_IDENTIFIER},\"cellTextDocuments\":[{TEXT_DOCUMENT_IDENTIFIER}]\\}");

    // Notebook document
    ctx.add_rule("NOTEBOOK_DOCUMENT", b"\\{\"uri\":\"{URI}\",\"notebookType\":\"{STRING_CONTENT}\",\"version\":{NUMBER},\"cells\":[{NOTEBOOK_CELL}]\\}");
    ctx.add_rule(
        "NOTEBOOK_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":\"{URI}\",\"version\":{NUMBER}\\}",
    );
    ctx.add_rule(
        "NOTEBOOK_DOCUMENT_CHANGE_EVENT",
        b"\\{\"cells\":{CELLS_CHANGE_EVENT}\\}",
    );
    ctx.add_rule("CELLS_CHANGE_EVENT", b"\\{\"structure\":{CELLS_STRUCTURE_CHANGE_EVENT},\"data\":[{NOTEBOOK_CELL}],\"textContent\":[{CELL_TEXT_CONTENT_CHANGE_EVENT}]\\}");
    ctx.add_rule("CELLS_STRUCTURE_CHANGE_EVENT", b"\\{\"array\":{ARRAY_CHANGE_EVENT},\"didOpen\":[{TEXT_DOCUMENT_ITEM}],\"didClose\":[{TEXT_DOCUMENT_IDENTIFIER}]\\}");
    ctx.add_rule(
        "ARRAY_CHANGE_EVENT",
        b"\\{\"start\":{NUMBER},\"deleteCount\":{NUMBER},\"cells\":[{NOTEBOOK_CELL}]\\}",
    );
    ctx.add_rule("CELL_TEXT_CONTENT_CHANGE_EVENT", b"\\{\"document\":{TEXT_DOCUMENT_IDENTIFIER},\"changes\":[{TEXT_DOCUMENT_CONTENT_CHANGE_EVENT}]\\}");
    ctx.add_rule(
        "NOTEBOOK_CELL",
        b"\\{\"kind\":{NUMBER},\"document\":\"{URI}\"\\}",
    );

    ctx
}

#[test]
fn test_grammar() {
    let mut grammar = get_grammar();
    grammar.initialize(10000);
}
