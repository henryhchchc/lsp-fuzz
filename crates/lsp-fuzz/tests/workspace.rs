use libafl::inputs::ToTargetBytesConverter;
use lsp_fuzz::{
    file_system::{FileSystemDirectory, FileSystemEntry},
    test_case::{
        HasWorkspace, LspInput, LspInputBytesConverter, WorkspaceEntry,
        messages::LspMessageSequence,
    },
    text_document::TextDocument,
    utf8::Utf8Input,
};
#[test]
fn converter_uses_the_materialized_workspace_directory() {
    use libafl_bolts::ToSlice;

    let root = tempfile::tempdir().unwrap();
    let input = LspInput {
        messages: LspMessageSequence::default(),
        workspace: FileSystemDirectory::from([(
            Utf8Input::new("main.rs".to_owned()),
            FileSystemEntry::File(WorkspaceEntry::SourceFile(TextDocument::new(
                lsp_fuzz_grammars::Language::Rust,
                b"fn main() {}".to_vec(),
            ))),
        )]),
    };
    let workspace = input.workspace_dir(root.path());
    use libafl::observers::Observer;
    let mut observer =
        lsp_fuzz::execution::workspace_observer::WorkspaceObserver::new(root.path().to_owned());
    observer.pre_exec(&mut (), &input).unwrap();
    assert_eq!(
        std::fs::read(workspace.join("main.rs")).unwrap(),
        b"fn main() {}"
    );

    let mut converter = LspInputBytesConverter::new(root.path().to_owned());
    let bytes = converter.convert_to_target_bytes(&mut (), &input);
    assert_eq!(bytes.to_slice(), input.request_bytes(&workspace));
    let payload = std::str::from_utf8(bytes.to_slice()).unwrap();
    assert!(payload.contains(&format!("file://{}/main.rs", workspace.display())));
    observer
        .post_exec(&mut (), &input, &libafl::executors::ExitKind::Ok)
        .unwrap();
    assert!(!workspace.exists());
}
