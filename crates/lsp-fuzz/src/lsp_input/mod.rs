use std::{borrow::Cow, fs::File, io::BufWriter, iter::once, path::Path, str::FromStr};

use derive_new::new as New;
use libafl::{
    corpus::CorpusId,
    generators::Generator,
    inputs::{BytesInput, HasTargetBytes, Input, UsesInput},
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand},
    HasMetadata,
};
use libafl_bolts::{rands::Rand, AsSlice, HasLen, Named};
use lsp_types::Uri;
use messages::LspMessages;
use serde::{Deserialize, Serialize};

use crate::{
    file_system::{FileSystemDirectory, FileSystemEntry},
    lsp::{self, capatibilities::fuzzer_client_capabilities, json_rpc::JsonRPCMessage},
    text_document::{GrammarBasedMutation, GrammarContextLookup, TextDocument},
    utf8::Utf8Input,
};

pub type FileContentInput = BytesInput;

pub mod messages;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct LspInput {
    pub messages: LspMessages,
    pub source_directory: FileSystemDirectory<TextDocument>,
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
        self.messages.len() + self.source_directory.len()
    }
}

impl LspInput {
    pub fn request_bytes(&self, workspace_dir: &Path) -> Vec<u8> {
        let init_request = lsp::Message::Initialize(lsp_types::InitializeParams {
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
        let Some((path, the_only_doc)) = self.source_directory.iter_files().next() else {
            unreachable!("We created only files");
        };
        let uri = Uri::from_str(&format!("lsp-fuzz://{}", path.display())).unwrap();
        let did_open_request = {
            lsp::Message::DidOpenTextDocument(lsp_types::DidOpenTextDocumentParams {
                text_document: lsp_types::TextDocumentItem {
                    uri: uri.clone(),
                    language_id: the_only_doc.language().lsp_language_id().to_owned(),
                    version: 1,
                    text: the_only_doc.to_string_lossy().into_owned(),
                },
            })
        };
        let shutdown = lsp::Message::Shutdown(());
        let mut bytes = Vec::new();
        let workspace_dir = workspace_dir
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string();
        for (id, request) in once(init_request)
            .chain(once(did_open_request))
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
        for (path, entry) in self.source_directory.iter() {
            let path = source_dir.join(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let FileSystemEntry::File(document) = entry else {
                todo!("We created only files currently")
            };
            std::fs::write(path, document.target_bytes().as_slice())?;
        }
        Ok(())
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
    S: UsesInput<Input = LspInput> + HasMetadata + HasCorpus + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        const MUTATE_DOCUMENT: bool = true;
        const MUTATE_REQUESTS: bool = false;
        match state.rand_mut().coinflip(0.5) {
            MUTATE_DOCUMENT => self.text_document_mutator.mutate(state, input),
            MUTATE_REQUESTS => self.requests_mutator.mutate(state, input),
        }
    }
}

#[derive(Debug, New)]
pub struct LspInpuGenerator<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl<S> Generator<LspInput, S> for LspInpuGenerator<'_>
where
    S: HasRand,
{
    fn generate(&mut self, state: &mut S) -> Result<LspInput, libafl::Error> {
        let rand = state.rand_mut();
        let (&language, grammar) =
            rand.choose(self.grammar_lookup.iter())
                .ok_or(libafl::Error::illegal_state(
                    "The grammar lookup context is empry",
                ))?;
        let ext = rand
            .choose(language.file_extensions())
            .ok_or(libafl::Error::illegal_state(
                "The language has no extensions",
            ))?;
        let single_file_name = format!("main.{ext}");
        let whole_programs = grammar
            .whole_programs()
            .map_err(|_| libafl::Error::illegal_state("The grammar has no whole programs"))?;
        let program = rand
            .choose(whole_programs)
            .ok_or(libafl::Error::illegal_state(
                "The grammar has no whole programs",
            ))?;
        Ok(LspInput {
            messages: LspMessages::default(),
            source_directory: FileSystemDirectory::from([(
                Utf8Input::new(single_file_name),
                FileSystemEntry::File(TextDocument::new(program.to_vec(), language)),
            )]),
        })
    }
}
