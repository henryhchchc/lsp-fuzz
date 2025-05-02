use libafl::nautilus::grammartec::context::Context;

use super::{LSPSpecMetaModel, META_MODEL_JSON};
use crate::lsp::metamodel::{BaseType, DataType};

fn convert_spec_grammar() -> Context {
    use super::MessageDirection::{Both, ClientToServer};

    let mut ctx = Context::new();
    let meta_model: LSPSpecMetaModel =
        serde_json::from_str(META_MODEL_JSON).expect("Fail to serialize");

    ctx.add_rule("Start", b"{Message}");

    // Numbers
    ctx.add_rule("Number", b"{Digit}");
    ctx.add_rule("Number", b"{Digit}{Number}");
    for i in 0..=9 {
        ctx.add_rule("Digit", &[b'0' + i]);
    }

    // Strings
    ctx.add_rule("String", b"\"{Char}\"");
    ctx.add_rule("String", b"{Char}{Char}");
    ctx.add_rule("Char", b"{Digit}");
    for letter in b'a'..=b'z' {
        ctx.add_rule("Char", &[letter]);
    }
    for letter in b'A'..=b'Z' {
        ctx.add_rule("Char", &[letter]);
    }
    ctx.add_rule("Char", b"_");
    ctx.add_rule("Char", b"-");
    ctx.add_rule("Char", b"/");
    ctx.add_rule("Char", b".");
    ctx.add_rule("Char", b",");
    ctx.add_rule("Char", b":");
    ctx.add_rule("Char", b"$");
    ctx.add_rule("Char", b"@");
    ctx.add_rule("Char", b"#");
    ctx.add_rule("Char", b"!");
    ctx.add_rule("Char", b"?");
    ctx.add_rule("Char", b"+");
    ctx.add_rule("Char", b"*");
    ctx.add_rule("Char", b"&");
    ctx.add_rule("Char", b"%");
    ctx.add_rule("Char", b"=");

    ctx.add_rule("MessageId", b"{Number}");
    ctx.add_rule("MessageId", b"{String}");

    ctx.add_rule("DocumentUri", b"file://{String}");

    ctx.add_rule("Uinteger", b"{Number}");
    ctx.add_rule("Integer", b"{Number}");
    ctx.add_rule("Integer", b"-{Number}");

    let client_to_server_messages = meta_model
        .requests
        .into_iter()
        .filter_map(|it| {
            matches!(it.message_direction, ClientToServer | Both).then_some((it.method, it.params))
        })
        .chain(meta_model.notifications.into_iter().filter_map(|it| {
            matches!(it.message_direction, ClientToServer | Both).then_some((it.method, it.params))
        }));

    for (method, param_type) in client_to_server_messages {
        let rule_format = if let Some(param_dt) = param_type {
            let DataType::Reference { name } = param_dt else {
                todo!()
            };
            let param = name;
            format!(
                "\\{{\"jsonrpc\":\"2.0\",\"id\":{{MessageId}},\"method\":\"{method}\",\"params\":{{TYPE_{param}}}\\}}",
            )
        } else {
            format!("\\{{\"jsonrpc\":\"2.0\",\"id\":{{MessageId}},\"method\":\"{method}\"\\}}",)
        };
        ctx.add_rule("Message", rule_format.into_bytes().as_slice());
    }

    for struct_t in meta_model.structures {
        let mut container = format!("\\{{{{GEN_{}_FIELDS}}", &struct_t.name);
        for ext in struct_t.extends {
            let DataType::Reference { name: ext_name } = ext else {
                todo!()
            };
            container.push_str(&format!("\\{{{{GEN_{ext_name}_FIELDS}}"));
        }
        for mixin in struct_t.mixins {
            let DataType::Reference { name: mixin_name } = mixin else {
                todo!()
            };
            container.push_str(&format!("\\{{{{GEN_{mixin_name}_FIELDS}}"));
        }
        container.push_str("\\}");
        ctx.add_rule(
            &format!("TYPE_{}", &struct_t.name),
            container.into_bytes().as_slice(),
        );
        let mut fields_container = "".to_string();
        for prop in struct_t.properties {
            let prop_name = format!("GEN_{}_PROP_{}", struct_t.name, prop.name);
            if !fields_container.is_empty() {
                fields_container.push(',');
            }
            fields_container.push_str(&format!("{{{prop_name}}}"));
            let prop_type_nt = match prop.data_type {
                DataType::Reference { name } => format!("TYPE_{name}"),
                DataType::Base(base) => base.name().to_string(),
                DataType::Array { element } => match element.as_ref() {
                    DataType::Reference { name } => format!("ARR_{name}"),
                    DataType::Base(base) => format!("ARR_{}", base.name()),
                    it => todo!("ARR: {it:?}"),
                },
                it => todo!("{it:?}"),
            };
            ctx.add_rule(
                &prop_name,
                format!("\"{}\":{{{prop_type_nt}}}", prop.name)
                    .into_bytes()
                    .as_slice(),
            );
            if prop.optional {
                ctx.add_rule(
                    &prop_name,
                    format!("\"{}\":null", prop.name).into_bytes().as_slice(),
                );
            }
        }
        ctx.add_rule(
            &format!("GEN_{}_FIELDS", struct_t.name),
            fields_container.into_bytes().as_slice(),
        );
    }

    ctx
}

#[test]
fn test_convert_spec_grammar() {
    let mut grammar = convert_spec_grammar();
    grammar.initialize(2333);
}

fn get_grammar() -> Context {
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
    ctx.add_rule("URI", b"\"file:///test/file.rs\"");
    ctx.add_rule(
        "POSITION",
        b"\\{\"line\":{NUMBER},\"character\":{NUMBER}\\}",
    );
    ctx.add_rule("RANGE", b"\\{\"start\":{POSITION},\"end\":{POSITION}\\}");

    // Workspace Structures
    ctx.add_rule(
        "WORKSPACE_SETTINGS",
        b"\\{\"rust-analyzer\":{RUST_SETTINGS}\\}",
    );
    ctx.add_rule("RUST_SETTINGS", b"\\{\"checkOnSave\":true\\}");
    ctx.add_rule("RUST_SETTINGS", b"\\{\"checkOnSave\":false\\}");

    // File Events
    ctx.add_rule("FILE_EVENT", b"\\{\"uri\":{URI},\"type\":1\\}");
    ctx.add_rule("FILE_EVENT", b"\\{\"uri\":{URI},\"type\":2\\}");
    ctx.add_rule("FILE_EVENT", b"\\{\"uri\":{URI},\"type\":3\\}");

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

    ctx
}

#[test]
fn test_grammar() {
    let mut grammar = get_grammar();
    grammar.initialize(10000);
}
