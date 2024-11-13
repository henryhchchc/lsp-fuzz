use std::{borrow::Cow, iter::once, path::Path, str::FromStr};

use fluent_uri::{
    component::{Authority, Scheme},
    encoding::EString,
    Uri,
};
use libafl::{
    corpus::CorpusId,
    generators::Generator,
    inputs::{BytesInput, HasTargetBytes, Input, UsesInput},
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand, State},
    HasMetadata,
};
use libafl_bolts::{AsSlice, HasLen, Named};
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};

use crate::{
    file_system::FileSystemEntryInput,
    lsp::{self, JsonRPCMessage},
    text_document::{GrammarBasedMutation, Language, TextDocument},
    utf8::Utf8Input,
};

pub type FileContentInput = BytesInput;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SourceDirectoryInput(pub OrderMap<Utf8Input, FileSystemEntryInput<TextDocument>>);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct LspMessages {
    pub messages: Vec<lsp::Message>,
}

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
        self.messages.messages.len()
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
            capabilities: lsp_types::ClientCapabilities {
                workspace: Some(lsp_types::WorkspaceClientCapabilities {
                    workspace_folders: Some(true),
                    ..Default::default()
                }),
                text_document: Some(lsp_types::TextDocumentClientCapabilities {
                    synchronization: Some(lsp_types::TextDocumentSyncClientCapabilities {
                        ..Default::default()
                    }),
                    publish_diagnostics: Some(lsp_types::PublishDiagnosticsClientCapabilities {
                        ..Default::default()
                    }),
                    diagnostic: Some(lsp_types::DiagnosticClientCapabilities {
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
        let Some((file_name, FileSystemEntryInput::File(the_only_doc))) =
            self.source_directory.0.iter().next()
        else {
            unreachable!("We created only files");
        };
        let mut path = EString::<fluent_uri::encoding::encoder::Path>::new();
        path.encode::<fluent_uri::encoding::encoder::Path>(
            workspace_dir
                .join(Path::new(file_name.as_str()))
                .to_string_lossy()
                .into_owned()
                .as_str(),
        );
        let uri = Uri::builder()
            .scheme(Scheme::new_or_panic("file"))
            .authority(Authority::EMPTY)
            .path(&path)
            .build()
            .unwrap();
        let uri = lsp_types::Uri::from_str(uri.to_string().as_str()).unwrap();
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
        let mut bytes = Vec::new();
        for (id, request) in self
            .messages
            .messages
            .iter()
            .cloned()
            .chain(once(init_request))
            .chain(once(did_open_request))
            .chain(once(inlay_hint))
            .enumerate()
        {
            let message = JsonRPCMessage::new(id, &request);
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
                unreachable!("We created only files")
            };
            std::fs::write(path, document.target_bytes().as_slice())?;
        }
        Ok(())
    }
}

#[derive(Debug, derive_more::Constructor)]
pub struct LspInputMutator<TM> {
    text_document_mutator: TM,
}

impl<TM> Named for LspInputMutator<TM> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("LspInputMutator");
        &NAME
    }
}

impl<TM, S> Mutator<LspInput, S> for LspInputMutator<TM>
where
    TM: Mutator<TextDocument, S>,
    S: State + UsesInput<Input = LspInput> + HasMetadata + HasCorpus + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let path = Utf8Input::new("main.c".to_owned());
        let SourceDirectoryInput(entries) = &mut input.source_directory;
        let FileSystemEntryInput::File(file_content) =
            entries.get_mut(&path).expect("This is the only file.")
        else {
            unreachable!("This is the only file.")
        };
        self.text_document_mutator.mutate(state, file_content)
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
            messages: LspMessages { messages: vec![] },
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
