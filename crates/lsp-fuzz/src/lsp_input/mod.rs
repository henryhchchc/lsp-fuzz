use std::{borrow::Cow, iter::once, path::Path, str::FromStr};

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
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};

use crate::{
    file_system::FileSystemEntryInput,
    lsp::{self, capatibilities::fuzzer_client_capabilities, json_rpc::JsonRPCMessage},
    text_document::{GrammarBasedMutation, Language, TextDocument},
    utf8::Utf8Input,
};

pub type FileContentInput = BytesInput;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SourceDirectoryInput(pub OrderMap<Utf8Input, FileSystemEntryInput<TextDocument>>);

pub mod messages;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct LspInput {
    pub messages: LspMessages,
    pub source_directory: SourceDirectoryInput,
}

impl Input for LspInput {
    fn generate_name(&self, id: Option<CorpusId>) -> String {
        format!(
            "input_{}",
            id.map(|it| it.to_string()).unwrap_or("unknown".to_owned())
        )
    }
}

impl HasLen for LspInput {
    fn len(&self) -> usize {
        self.messages.len()
            + self
                .source_directory
                .0
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>()
    }
}

impl LspInput {
    pub fn request_bytes(&self, workspace_dir: &Path) -> Vec<u8> {
        let init_request = lsp::Message::Initialize(lsp_types::InitializeParams {
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: lsp_types::Uri::from_str(&format!("file://{}", workspace_dir.display()))
                    .unwrap(),
                name: workspace_dir
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            }]),
            capabilities: fuzzer_client_capabilities(),
            ..Default::default()
        });
        let Some((file_name, FileSystemEntryInput::File(the_only_doc))) =
            self.source_directory.0.iter().next()
        else {
            unreachable!("We created only files");
        };
        let uri = Uri::from_str(&format!("workspace://{}", file_name.as_str())).unwrap();
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
        let inlay_hint = lsp::Message::InlayHintRequest(lsp_types::InlayHintParams {
            text_document: lsp_types::TextDocumentIdentifier { uri },
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 1,
                },
                end: lsp_types::Position {
                    line: 1000,
                    character: 1,
                },
            },
            work_done_progress_params: Default::default(),
        });
        let shutdown = lsp::Message::Shutdown(());
        let exit = lsp::Message::Exit(());
        let mut bytes = Vec::new();
        for (id, request) in once(init_request)
            .chain(once(did_open_request))
            .chain(once(inlay_hint))
            .chain(self.messages.iter().cloned())
            .chain(once(shutdown))
            .chain(once(exit))
            .enumerate()
        {
            let (method, params) = request.as_json();
            let message = JsonRPCMessage::new(id, method.into(), params);
            bytes.extend(message.to_lsp_payload());
        }
        bytes
    }

    pub fn setup_source_dir(&self, source_dir: &Path) -> Result<(), std::io::Error> {
        for (path, entry) in self.source_directory.0.iter() {
            let path = source_dir.join(path.as_str());
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let FileSystemEntryInput::File(document) = entry else {
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

#[derive(Debug)]
pub struct LspInpuGenerator;

impl<S> Generator<LspInput, S> for LspInpuGenerator
where
    S: HasRand,
{
    fn generate(&mut self, _state: &mut S) -> Result<LspInput, libafl::Error> {
        Ok(LspInput {
            messages: LspMessages::default(),
            source_directory: SourceDirectoryInput(OrderMap::from([(
                Utf8Input::new("main.c".to_owned()),
                FileSystemEntryInput::File(TextDocument::new(
                    b"int main() { return 0; }".to_vec(),
                    Language::C,
                )),
            )])),
        })
    }
}
