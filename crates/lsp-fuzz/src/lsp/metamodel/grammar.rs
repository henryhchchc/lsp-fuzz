use libafl::nautilus::grammartec::context::Context;

pub fn get_grammar() -> Context {
    let mut ctx = Context::new();

    // Core message structure
    ctx.add_rule("START", b"{REQUEST}");
    ctx.add_rule("START", b"{NOTIFICATION}");

    // Basic message types
    ctx.add_rule("REQUEST", b"\\{\"jsonrpc\":\"2.0\",\"id\":{NUMBER},\"method\":{CLIENT_REQUEST_METHOD},\"params\":{PARAMS}\\}");
    ctx.add_rule(
        "NOTIFICATION",
        b"\\{\"jsonrpc\":\"2.0\",\"method\":{CLIENT_NOTIFICATION_METHOD},\"params\":{PARAMS}\\}",
    );

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

    // Basic JSON structures
    // ctx.add_rule("PARAMS", b"{JSON_OBJECT}");
    // ctx.add_rule("PARAMS", b"{JSON_ARRAY}");

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

    // Client Request Methods
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"initialize\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"shutdown\"");
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/willSaveWaitUntil\"",
    );
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/completion\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"completionItem/resolve\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/hover\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/signatureHelp\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/declaration\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/definition\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/typeDefinition\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/implementation\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/references\"");
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/documentHighlight\"",
    );
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/documentSymbol\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/codeAction\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"codeAction/resolve\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/codeLens\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"codeLens/resolve\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/documentLink\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"documentLink/resolve\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/documentColor\"");
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/colorPresentation\"",
    );
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/formatting\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/rangeFormatting\"");
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/onTypeFormatting\"",
    );
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/rename\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/prepareRename\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/foldingRange\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/selectionRange\"");
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/prepareCallHierarchy\"",
    );
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"callHierarchy/incomingCalls\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"callHierarchy/outgoingCalls\"");
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/semanticTokens/full\"",
    );
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/semanticTokens/full/delta\"",
    );
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/semanticTokens/range\"",
    );
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"workspace/symbol\"");
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"workspace/executeCommand\"");
    ctx.add_rule(
        "CLIENT_REQUEST_METHOD",
        b"\"textDocument/linkedEditingRange\"",
    );
    ctx.add_rule("CLIENT_REQUEST_METHOD", b"\"textDocument/moniker\"");

    // Client Notification Methods
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"initialized\"");
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"exit\"");
    ctx.add_rule(
        "CLIENT_NOTIFICATION_METHOD",
        b"\"workspace/didChangeConfiguration\"",
    );
    ctx.add_rule(
        "CLIENT_NOTIFICATION_METHOD",
        b"\"workspace/didChangeWatchedFiles\"",
    );
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"textDocument/didOpen\"");
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"textDocument/didChange\"");
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"textDocument/willSave\"");
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"textDocument/didSave\"");
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"textDocument/didClose\"");
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"$/cancelRequest\"");
    ctx.add_rule("CLIENT_NOTIFICATION_METHOD", b"\"$/progress\"");
    ctx.add_rule(
        "CLIENT_NOTIFICATION_METHOD",
        b"\"workspace/didCreateFiles\"",
    );
    ctx.add_rule(
        "CLIENT_NOTIFICATION_METHOD",
        b"\"workspace/didRenameFiles\"",
    );
    ctx.add_rule(
        "CLIENT_NOTIFICATION_METHOD",
        b"\"workspace/didDeleteFiles\"",
    );

    // Common Parameters
    ctx.add_rule("PARAMS", b"{INITIALIZE_PARAMS}");
    ctx.add_rule("PARAMS", b"{TEXT_DOCUMENT_PARAMS}");
    ctx.add_rule("PARAMS", b"{WORKSPACE_PARAMS}");
    ctx.add_rule("PARAMS", b"null");

    // Initialize Parameters
    ctx.add_rule("INITIALIZE_PARAMS", b"\\{\"processId\":{NUMBER},\"clientInfo\":{CLIENT_INFO},\"rootUri\":{URI},\"capabilities\":{CLIENT_CAPABILITIES}\\}");
    ctx.add_rule(
        "CLIENT_INFO",
        b"\\{\"name\":\"test-client\",\"version\":\"1.0.0\"\\}",
    );

    // Text Document Parameters
    ctx.add_rule(
        "TEXT_DOCUMENT_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER}\\}",
    );
    ctx.add_rule(
        "TEXT_DOCUMENT_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION}\\}",
    );
    ctx.add_rule(
        "TEXT_DOCUMENT_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE}\\}",
    );

    // Workspace Parameters
    ctx.add_rule(
        "WORKSPACE_PARAMS",
        b"\\{\"settings\":{WORKSPACE_SETTINGS}\\}",
    );
    ctx.add_rule("WORKSPACE_PARAMS", b"\\{\"changes\":[{FILE_EVENT}]\\}");

    // Common Structures
    ctx.add_rule(
        "TEXT_DOCUMENT_IDENTIFIER",
        b"\\{\"uri\":{URI},\"version\":{NUMBER}\\}",
    );
    ctx.add_rule("URI", b"\"file://{STRING}\"");
    ctx.add_rule(
        "POSITION",
        b"\\{\"line\":{NUMBER},\"character\":{NUMBER}\\}",
    );
    ctx.add_rule("RANGE", b"\\{\"start\":{POSITION},\"end\":{POSITION}\\}");

    // File Events
    ctx.add_rule("FILE_EVENT", b"\\{\"uri\":{URI},\"type\":{NUMBER}\\}");

    // Client Capabilities
    ctx.add_rule("CLIENT_CAPABILITIES", b"\\{\"workspace\":{WORKSPACE_CLIENT_CAPABILITIES},\"textDocument\":{TEXT_DOCUMENT_CLIENT_CAPABILITIES}\\}");
    ctx.add_rule(
        "WORKSPACE_CLIENT_CAPABILITIES",
        b"\\{\"applyEdit\":true,\"didChangeConfiguration\":\\{\"dynamicRegistration\":true\\}\\}",
    );
    ctx.add_rule(
        "TEXT_DOCUMENT_CLIENT_CAPABILITIES",
        b"\\{\"synchronization\":{SYNC_CAPABILITIES},\"completion\":{COMPLETION_CAPABILITIES}\\}",
    );
    ctx.add_rule("SYNC_CAPABILITIES", b"\\{\"dynamicRegistration\":true,\"willSave\":true,\"willSaveWaitUntil\":true,\"didSave\":true\\}");
    ctx.add_rule(
        "COMPLETION_CAPABILITIES",
        b"\\{\"dynamicRegistration\":true,\"completionItem\":\\{\"snippetSupport\":true\\}\\}",
    );

    // Workspace Settings
    ctx.add_rule("WORKSPACE_SETTINGS", b"\\{\"{STRING}\":{JSON_OBJECT}\\}");

    // Add more specific parameter types
    ctx.add_rule("PARAMS", b"{COMPLETION_PARAMS}");
    ctx.add_rule("PARAMS", b"{DOCUMENT_SYMBOL_PARAMS}");
    ctx.add_rule("PARAMS", b"{CODE_ACTION_PARAMS}");
    ctx.add_rule("PARAMS", b"{RENAME_PARAMS}");
    ctx.add_rule("PARAMS", b"{FORMATTING_PARAMS}");
    ctx.add_rule("PARAMS", b"{DID_CHANGE_PARAMS}");

    // Completion parameters
    ctx.add_rule("COMPLETION_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"context\":{COMPLETION_CONTEXT}\\}");
    ctx.add_rule("COMPLETION_CONTEXT", b"\\{\"triggerKind\":{NUMBER}\\}");

    // Document Symbol parameters
    ctx.add_rule(
        "DOCUMENT_SYMBOL_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER}\\}",
    );

    // Code Action parameters
    ctx.add_rule("CODE_ACTION_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"range\":{RANGE},\"context\":{CODE_ACTION_CONTEXT}\\}");
    ctx.add_rule("CODE_ACTION_CONTEXT", b"\\{\"diagnostics\":{JSON_ARRAY}\\}");

    // Rename parameters
    ctx.add_rule("RENAME_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"position\":{POSITION},\"newName\":\"{STRING_CONTENT}\"\\}");

    // Formatting parameters
    ctx.add_rule(
        "FORMATTING_PARAMS",
        b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"options\":{FORMATTING_OPTIONS}\\}",
    );
    ctx.add_rule(
        "FORMATTING_OPTIONS",
        b"\\{\"tabSize\":{NUMBER},\"insertSpaces\":true\\}",
    );

    // Document change parameters
    ctx.add_rule("DID_CHANGE_PARAMS", b"\\{\"textDocument\":{TEXT_DOCUMENT_IDENTIFIER},\"contentChanges\":[{TEXT_DOCUMENT_CONTENT_CHANGE_EVENT}]\\}");
    ctx.add_rule(
        "TEXT_DOCUMENT_CONTENT_CHANGE_EVENT",
        b"\\{\"text\":\"{STRING_CONTENT}\"\\}",
    );
    ctx.add_rule(
        "TEXT_DOCUMENT_CONTENT_CHANGE_EVENT",
        b"\\{\"range\":{RANGE},\"text\":\"{STRING_CONTENT}\"\\}",
    );

    ctx
}

#[test]
fn test_grammar() {
    let mut grammar = get_grammar();
    grammar.initialize(10000);
}
