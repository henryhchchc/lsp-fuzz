use std::{
    borrow::Cow,
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
    io::BufWriter,
    iter::once,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

use derive_new::new as New;
use libafl::{
    HasMetadata,
    corpus::CorpusId,
    generators::Generator,
    inputs::{BytesInput, HasTargetBytes, Input, InputToBytes},
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand},
};
use libafl_bolts::{HasLen, Named, ownedref::OwnedSlice, rands::Rand};
use lsp_fuzz_grammars::Language;
use lsp_types::{ClientInfo, InitializedParams, TraceValue, Uri};
use messages::LspMessages;
use serde::{Deserialize, Serialize};

use crate::{
    file_system::{FileSystemDirectory, FileSystemEntry},
    lsp::{self, capabilities::fuzzer_client_capabilities},
    text_document::{
        GrammarBasedMutation, TextDocument,
        generation::{GrammarContextLookup, NamedNodeGenerator, RandomRuleSelectionStrategy},
    },
    utf8::Utf8Input,
    utils::AflContext,
};

pub type FileContentInput = BytesInput;

pub mod messages;
pub mod ops_curiosity;

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

impl LspInput {
    pub const NAME_PREFIX: &str = "input_";
    pub const PROROCOL_PREFIX: &str = "lsp-fuzz://";

    pub fn root_uri() -> Uri {
        static WORKSPACE_ROOT_URI: LazyLock<lsp_types::Uri> =
            LazyLock::new(|| LspInput::PROROCOL_PREFIX.parse().unwrap());
        WORKSPACE_ROOT_URI.clone()
    }

    pub fn workspace_hash(&self) -> u64 {
        let mut hasher = ahash::AHasher::default();
        self.workspace.hash(&mut hasher);
        hasher.finish()
    }

    /// Retrieves a text document from the workspace by its URI.
    ///
    /// This function looks up a text document in the workspace using the provided URI.
    /// The URI must have the prefix specified by `LspInput::PROROCOL_PREFIX` as it maps to
    /// the fuzzer's virtual file system.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI of the text document to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&TextDocument)` - The found text document
    /// * `None` - If no text document exists at the given URI or if the entry is not a source file
    ///
    pub fn get_text_document(&self, uri: &lsp_types::Uri) -> Option<&TextDocument> {
        let path = uri
            .as_str()
            .strip_prefix(LspInput::PROROCOL_PREFIX)
            .expect("The URI must start with fuzzer uri");
        if let Some(FileSystemEntry::File(WorkspaceEntry::SourceFile(doc))) =
            self.workspace.get(path)
        {
            Some(doc)
        } else {
            None
        }
    }
}

impl Input for LspInput {
    fn generate_name(&self, id: Option<CorpusId>) -> String {
        let id_str = id.map(|it| it.to_string()).unwrap_or_else(|| {
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            format!("h_{}", hasher.finish())
        });
        format!("{}{}", Self::NAME_PREFIX, id_str)
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

#[derive(Debug, New)]
pub struct LspInputBytesConverter {
    workspace_root: PathBuf,
}

impl InputToBytes<LspInput> for LspInputBytesConverter {
    fn to_bytes<'a>(&mut self, input: &'a LspInput) -> OwnedSlice<'a, u8> {
        let input_hash = input.workspace_hash();
        let workspace_dir = self
            .workspace_root
            .join(format!("lsp-fuzz-workspace_{input_hash}"));
        input.request_bytes(&workspace_dir).into()
    }
}

impl LspInput {
    pub fn request_bytes(&self, workspace_dir: &Path) -> Vec<u8> {
        let message_sequence = self.message_sequence();

        let workspace_dir = workspace_dir
            .to_str()
            .expect("`workspace_dir` does not contain valid UTF-8");
        let workspace_dir = if workspace_dir.ends_with('/') {
            Cow::Borrowed(workspace_dir)
        } else {
            Cow::Owned(format!("{workspace_dir}/"))
        };
        let workspace_uri = format!("file://{workspace_dir}");

        let mut id = 0;
        let bytes: Vec<_> = message_sequence
            .flat_map(|msg| {
                let message = msg.into_json_rpc(&mut id, Some(&workspace_uri));
                message.to_lsp_payload()
            })
            .collect();

        bytes
    }

    pub fn message_sequence(&self) -> impl Iterator<Item = lsp::ClientToServerMessage> + use<'_> {
        #[allow(
            deprecated,
            reason = "Some language servers (e.g., rust-analyzer) still rely on `root_uri`."
        )]
        let init_request = lsp::ClientToServerMessage::Initialize(lsp_types::InitializeParams {
            process_id: None,
            client_info: Some(ClientInfo {
                name: env!("CARGO_PKG_NAME").to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            root_uri: Some(Self::root_uri()),
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: Self::root_uri(),
                name: "default_workspace".to_owned(),
            }]),
            capabilities: fuzzer_client_capabilities(),
            trace: Some(TraceValue::Off),
            ..Default::default()
        });
        let initialized_req = lsp::ClientToServerMessage::Initialized(InitializedParams {});

        let did_open_notifications = self
            .workspace
            .iter_files()
            .filter_map(|(path, entry)| entry.as_source_file().map(|doc| (path, doc)))
            .map(|(path, doc)| {
                let path_str = path.to_str().expect("Path should contain valid UTF-8");
                let uri =
                    Uri::from_str(&format!("{}{}", LspInput::PROROCOL_PREFIX, path_str)).unwrap();
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
        let exit = lsp::ClientToServerMessage::Exit(());

        once(init_request)
            .chain(once(initialized_req))
            .chain(did_open_notifications)
            .chain(self.messages.iter().cloned())
            .chain(once(shutdown))
            .chain(once(exit))
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

impl<TM, RM, State> Mutator<LspInput, State> for LspInputMutator<TM, RM>
where
    TM: Mutator<LspInput, State>,
    RM: Mutator<LspInput, State>,
    State: HasMetadata + HasCorpus<LspInput> + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
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
        state: &mut State,
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

impl<State> Generator<LspInput, State> for LspInputGenerator<'_>
where
    State: HasRand,
{
    fn generate(&mut self, state: &mut State) -> Result<LspInput, libafl::Error> {
        let rand = state.rand_mut();
        let grammar = rand
            .choose(self.grammar_lookup.iter())
            .afl_context("The grammar lookup context is empry")?;
        let language = grammar.language();
        let ext = rand
            .choose(language.file_extensions())
            .afl_context("The language has no extensions")?;
        let document_content = loop {
            let selection_strategy = RandomRuleSelectionStrategy;
            let generator = NamedNodeGenerator::new(grammar, selection_strategy);
            let generate_node = generator.generate(grammar.start_symbol(), state);
            if let Ok(code) = generate_node {
                break code;
            }
        };
        let mut text_document = TextDocument::new(language, document_content.to_vec());
        text_document.update_metadata();

        let workspace = match language {
            Language::Rust => rust_workspace(text_document, ext),
            _ => main_file_workspace(text_document, ext),
        };
        Ok(LspInput {
            messages: LspMessages::default(),
            workspace,
        })
    }
}

fn main_file_workspace(doc: TextDocument, extension: &str) -> FileSystemDirectory<WorkspaceEntry> {
    FileSystemDirectory::from([(
        Utf8Input::new(format!("main.{extension}")),
        FileSystemEntry::File(WorkspaceEntry::SourceFile(doc)),
    )])
}

// const CARGO_TOML: &str = r#"
// [package]
// name = "test_pkg"
// version = "0.1.0"
// edition = "2021"

// [dependencies]
// "#;

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

fn rust_workspace(doc: TextDocument, _extension: &str) -> FileSystemDirectory<WorkspaceEntry> {
    FileSystemDirectory::from([
        // (
        //     Utf8Input::new("Cargo.toml".to_owned()),
        //     FileSystemEntry::File(WorkspaceEntry::Skeleton(CARGO_TOML.as_bytes().to_vec())),
        // ),
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
