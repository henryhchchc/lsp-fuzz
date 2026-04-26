use std::{iter::once, path::Path};

use lsp_fuzz_grammars::Language;
use lsp_types::{ClientInfo, InitializedParams, TraceValue};

use super::{LspInput, WorkspaceEntry, uri};
use crate::{
    file_system::{FileSystemDirectory, FileSystemEntry},
    lsp::{self, capabilities::fuzzer_client_capabilities},
    text_document::{GrammarBasedMutation, TextDocument},
    utf8::Utf8Input,
};

pub fn request_bytes(input: &LspInput, workspace_dir: &Path) -> Vec<u8> {
    let workspace_dir =
        uri::workspace_uri(workspace_dir).expect("`workspace_dir` does not contain valid UTF-8");
    let workspace_uri = format!("file://{workspace_dir}");

    let mut id = 0;
    message_sequence(input)
        .flat_map(|msg| {
            let message = msg.into_json_rpc(&mut id, Some(&workspace_uri));
            message.to_lsp_payload()
        })
        .collect()
}

pub fn message_sequence(input: &LspInput) -> impl Iterator<Item = lsp::LspMessage> + use<'_> {
    #[allow(
        deprecated,
        reason = "Some language servers (e.g., rust-analyzer) still rely on `root_uri`."
    )]
    let init_request = lsp::LspMessage::Initialize(lsp_types::InitializeParams {
        process_id: None,
        client_info: Some(ClientInfo {
            name: env!("CARGO_PKG_NAME").to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
        root_uri: Some(LspInput::root_uri()),
        workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
            uri: LspInput::root_uri(),
            name: "default_workspace".to_owned(),
        }]),
        capabilities: fuzzer_client_capabilities(),
        trace: Some(TraceValue::Off),
        ..Default::default()
    });
    let initialized_req = lsp::LspMessage::Initialized(InitializedParams {});

    let did_open_notifications = input
        .workspace
        .iter_files()
        .filter_map(|(path, entry)| entry.as_source_file().map(|doc| (path, doc)))
        .map(|(path, doc)| {
            let uri = uri::virtual_uri_for_path(&path).expect("Path should contain valid UTF-8");
            lsp::LspMessage::DidOpenTextDocument(lsp_types::DidOpenTextDocumentParams {
                text_document: lsp_types::TextDocumentItem {
                    uri: uri.clone(),
                    language_id: doc.language().lsp_language_id().to_owned(),
                    version: 1,
                    text: doc.to_string_lossy().into_owned(),
                },
            })
        });
    let shutdown = lsp::LspMessage::Shutdown(());
    let exit = lsp::LspMessage::Exit(());

    once(init_request)
        .chain(once(initialized_req))
        .chain(did_open_notifications)
        .chain(input.messages.iter().cloned())
        .chain(once(shutdown))
        .chain(once(exit))
}

pub fn workspace_for_document(
    language: Language,
    doc: TextDocument,
    extension: &str,
) -> FileSystemDirectory<WorkspaceEntry> {
    match language {
        Language::Rust => rust_workspace(doc),
        _ => main_file_workspace(doc, extension),
    }
}

fn main_file_workspace(doc: TextDocument, extension: &str) -> FileSystemDirectory<WorkspaceEntry> {
    FileSystemDirectory::from([(
        Utf8Input::new(format!("main.{extension}")),
        FileSystemEntry::File(WorkspaceEntry::SourceFile(doc)),
    )])
}

// rust-analyzer runs faster when configured with a `rust-project.json` file.
const RUST_PROJECT_JSON: &str = r#"
{
    "sysroot_src": "/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library",
    "crates": [
        {
            "root_module": "src/lib.rs",
            "edition": "2021",
            "deps": [],
        },
    ]
}
"#;

fn rust_workspace(doc: TextDocument) -> FileSystemDirectory<WorkspaceEntry> {
    FileSystemDirectory::from([
        (
            Utf8Input::new("rust-project.json".to_owned()),
            FileSystemEntry::File(WorkspaceEntry::Skeleton(
                RUST_PROJECT_JSON.as_bytes().to_vec(),
            )),
        ),
        (
            Utf8Input::new("src".to_owned()),
            FileSystemEntry::Directory(FileSystemDirectory::from([(
                Utf8Input::new("lib.rs".to_owned()),
                FileSystemEntry::File(WorkspaceEntry::SourceFile(doc)),
            )])),
        ),
    ])
}
