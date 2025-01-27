use std::{borrow::Cow, fs::File, io::BufWriter, iter::once, path::Path, str::FromStr};

use derive_new::new as New;
use libafl::{
    corpus::CorpusId,
    generators::Generator,
    inputs::{BytesInput, HasTargetBytes, Input},
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand},
    HasMetadata,
};
use libafl_bolts::{ownedref::OwnedSlice, rands::Rand, HasLen, Named};
use lsp_types::{InitializedParams, Uri};
use messages::LspMessages;
use serde::{Deserialize, Serialize};

use crate::{
    file_system::{FileSystemDirectory, FileSystemEntry},
    lsp::{self, capabilities::fuzzer_client_capabilities, json_rpc::JsonRPCMessage},
    text_document::{GrammarBasedMutation, GrammarContextLookup, Language, TextDocument},
    utf8::Utf8Input,
    utils::AflContext,
};

pub type FileContentInput = BytesInput;

pub mod messages;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceEntry {
    SourceFile(TextDocument),
    Skeleton(Vec<u8>),
}

impl WorkspaceEntry {
    pub const fn as_source_file(&self) -> Option<&TextDocument> {
        if let WorkspaceEntry::SourceFile(doc) = self {
            Some(doc)
        } else {
            None
        }
    }

    pub const fn as_source_file_mut(&mut self) -> Option<&mut TextDocument> {
        if let WorkspaceEntry::SourceFile(doc) = self {
            Some(doc)
        } else {
            None
        }
    }

    pub fn as_skeleton(&self) -> Option<&[u8]> {
        if let WorkspaceEntry::Skeleton(bytes) = self {
            Some(bytes.as_slice())
        } else {
            None
        }
    }

    pub const fn as_skeleton_mut(&mut self) -> Option<&mut Vec<u8>> {
        if let WorkspaceEntry::Skeleton(bytes) = self {
            Some(bytes)
        } else {
            None
        }
    }
}

impl HasLen for WorkspaceEntry {
    fn len(&self) -> usize {
        match self {
            WorkspaceEntry::SourceFile(doc) => doc.len(),
            WorkspaceEntry::Skeleton(bytes) => bytes.len(),
        }
    }
}

impl HasTargetBytes for WorkspaceEntry {
    fn target_bytes(&self) -> OwnedSlice<'_, u8> {
        match self {
            WorkspaceEntry::SourceFile(doc) => doc.target_bytes(),
            WorkspaceEntry::Skeleton(bytes) => bytes.as_slice().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct LspInput {
    pub messages: LspMessages,
    pub workspace: FileSystemDirectory<WorkspaceEntry>,
}

impl Input for LspInput {
    fn generate_name(&self, id: Option<CorpusId>) -> String {
        format!(
            "input_{}",
            id.map(|it| it.to_string()).unwrap_or("unknown".to_owned())
        )
    }

    fn to_file<P>(&self, path: P) -> Result<(), libafl::Error>
    where
        P: AsRef<Path>,
    {
        let file = File::create(path)?;
        let buf_writer = BufWriter::new(file);
        serde_cbor::to_writer(buf_writer, self)
            .map_err(|e| libafl::Error::serialize(format!("{e:#?}")))
    }

    fn from_file<P>(path: P) -> Result<Self, libafl::Error>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let buf_reader = std::io::BufReader::new(file);
        serde_cbor::from_reader(buf_reader).map_err(|e| libafl::Error::serialize(format!("{e:#?}")))
    }
}

impl HasLen for LspInput {
    fn len(&self) -> usize {
        self.messages.len() + self.workspace.len()
    }
}

impl LspInput {
    pub fn request_bytes(&self, workspace_dir: &Path) -> Vec<u8> {
        #[allow(deprecated, reason = "rust-analyzer uses root_uri")]
        let init_request = lsp::ClientToServerMessage::Initialize(lsp_types::InitializeParams {
            root_uri: Some("lsp-fuzz://".parse().unwrap()),
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: "lsp-fuzz://".parse().unwrap(),
                name: workspace_dir
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            }]),
            capabilities: fuzzer_client_capabilities(),
            ..Default::default()
        });
        let initialized_req = lsp::ClientToServerMessage::Initialized(InitializedParams {});

        let did_open_notifications = self
            .workspace
            .iter_files()
            .filter_map(|(path, entry)| entry.as_source_file().map(|doc| (path, doc)))
            .map(|(path, doc)| {
                let uri = Uri::from_str(&format!("lsp-fuzz://{}", path.display())).unwrap();
                lsp::ClientToServerMessage::DidOpenTextDocument(
                    lsp_types::DidOpenTextDocumentParams {
                        text_document: lsp_types::TextDocumentItem {
                            uri: uri.clone(),
                            language_id: doc.language().lsp_language_id().to_owned(),
                            version: 1,
                            text: doc.to_string_lossy().into_owned(),
                        },
                    },
                )
            });
        let shutdown = lsp::ClientToServerMessage::Shutdown(());
        let mut bytes = Vec::new();
        let workspace_dir = workspace_dir
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string();
        for (id, request) in once(init_request)
            .chain(once(initialized_req))
            .chain(did_open_notifications)
            .chain(self.messages.iter().cloned())
            .chain(once(shutdown))
            .map(|it| it.with_workspace_dir(&workspace_dir))
            .enumerate()
        {
            let id = Some(id).filter(|_| request.is_request());
            let (method, params) = request.as_json();
            let message = JsonRPCMessage::new(id, method.into(), params);
            bytes.extend(message.to_lsp_payload());
        }
        bytes
    }

    pub fn setup_source_dir(&self, source_dir: &Path) -> Result<(), std::io::Error> {
        self.workspace.write_to_fs(source_dir)
    }
}

#[derive(Debug, derive_more::Constructor)]
pub struct LspInputMutator<TM, RM> {
    text_document_mutator: TM,
    requests_mutator: RM,
}

impl<TM, RM> Named for LspInputMutator<TM, RM> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("LspInputMutator");
        &NAME
    }
}

impl<TM, RM, S> Mutator<LspInput, S> for LspInputMutator<TM, RM>
where
    TM: Mutator<LspInput, S>,
    RM: Mutator<LspInput, S>,
    S: HasMetadata + HasCorpus<LspInput> + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let mut result = MutationResult::Skipped;
        if self.text_document_mutator.mutate(state, input)? == MutationResult::Mutated {
            result = MutationResult::Mutated;
        }
        if self.requests_mutator.mutate(state, input)? == MutationResult::Mutated {
            result = MutationResult::Mutated;
        }
        Ok(result)
    }

    fn post_exec(
        &mut self,
        state: &mut S,
        new_corpus_id: Option<CorpusId>,
    ) -> Result<(), libafl::Error> {
        self.text_document_mutator.post_exec(state, new_corpus_id)?;
        self.requests_mutator.post_exec(state, new_corpus_id)?;
        Ok(())
    }
}

#[derive(Debug, New)]
pub struct LspInputGenerator<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl<S> Generator<LspInput, S> for LspInputGenerator<'_>
where
    S: HasRand,
{
    fn generate(&mut self, state: &mut S) -> Result<LspInput, libafl::Error> {
        let rand = state.rand_mut();
        let (&language, grammar) = rand
            .choose(self.grammar_lookup.iter())
            .afl_context("The grammar lookup context is empry")?;
        let ext = rand
            .choose(language.file_extensions())
            .afl_context("The language has no extensions")?;
        let whole_programs = grammar
            .start_symbol_fragments()
            .afl_context("The grammar has no whole programs")?;
        let document_content = rand
            .choose(whole_programs)
            .afl_context("The grammar has no whole programs")?;
        let mut text_document = TextDocument::new(document_content.to_vec(), language);
        text_document.generate_parse_tree(grammar);

        let workspace = match language {
            Language::C | Language::CPlusPlus => c_workspace(text_document, ext),
            Language::Rust => rust_workspace(text_document, ext),
        };
        Ok(LspInput {
            messages: LspMessages::default(),
            workspace,
        })
    }
}

fn c_workspace(doc: TextDocument, extension: &str) -> FileSystemDirectory<WorkspaceEntry> {
    FileSystemDirectory::from([(
        Utf8Input::new(format!("main.{extension}")),
        FileSystemEntry::File(WorkspaceEntry::SourceFile(doc)),
    )])
}

const CARGO_TOML: &str = r#"
[package]
name = "test_pkg"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

fn rust_workspace(doc: TextDocument, _extension: &str) -> FileSystemDirectory<WorkspaceEntry> {
    FileSystemDirectory::from([
        (
            Utf8Input::new("Cargo.toml".to_owned()),
            FileSystemEntry::File(WorkspaceEntry::Skeleton(CARGO_TOML.as_bytes().to_vec())),
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
