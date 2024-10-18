use std::{collections::HashMap, iter::once, path::Path, str::FromStr};

use file_system::FileSystemEntryInput;
use fluent_uri::{
    component::{Authority, Scheme},
    encoding::EString,
    Uri,
};
use libafl::{
    corpus::CorpusId,
    inputs::{BytesInput, HasMutatorBytes, Input},
};
use libafl_bolts::HasLen;
use lsp::encapsulate_request_content;
use serde::{Deserialize, Serialize};

use crate::utf8::Utf8Input;

pub mod file_system;
pub mod lsp;

pub type FileContentInput = BytesInput;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceDirectoryInput(pub HashMap<Utf8Input, FileSystemEntryInput<FileContentInput>>);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspRequestSequence {
    pub requests: Vec<lsp::LspRequest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspInput {
    pub requests: LspRequestSequence,
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
        self.requests.requests.len()
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
        let init_request = lsp::LspRequest::Initialize(lsp_types::InitializeParams {
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: lsp_types::Uri::from_str(&format!("file://{}", workspace_dir.display()))
                    .unwrap(),
                name: workspace_dir
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            }]),
            ..Default::default()
        });
        let did_open_requests = self.source_directory.0.keys().map(|source_file| {
            let mut path = EString::<fluent_uri::encoding::encoder::Path>::new();
            path.encode::<fluent_uri::encoding::encoder::Path>(
                workspace_dir
                    .join(Path::new(source_file.as_str()))
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
            lsp::LspRequest::DidOpenTextDocument(lsp_types::DidOpenTextDocumentParams {
                text_document: lsp_types::TextDocumentItem {
                    uri,
                    language_id: "c".to_string(),
                    version: 1,
                    text: "".to_string(),
                },
            })
        });
        let mut bytes = Vec::new();
        for (id, request) in self
            .requests
            .requests
            .iter()
            .cloned()
            .chain(once(init_request))
            .chain(did_open_requests)
            .enumerate()
        {
            bytes.extend_from_slice(&encapsulate_request_content(&request.as_json(id + 1)));
        }
        bytes
    }

    pub fn setup_source_dir(&self, source_dir: &Path) -> Result<(), std::io::Error> {
        for (path, entry) in self.source_directory.0.iter() {
            let path = source_dir.join(path.as_str());
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let FileSystemEntryInput::File(content) = entry else {
                unreachable!("We created only files")
            };
            std::fs::write(path, content.bytes())?;
        }
        Ok(())
    }
}
