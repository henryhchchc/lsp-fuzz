use std::{
    collections::HashMap,
    iter::once,
    path::{Path, PathBuf},
    str::FromStr,
};

use fluent_uri::{
    component::{Authority, Scheme},
    encoding::{EStr, EString},
    Uri,
};
use libafl::{
    corpus::CorpusId,
    inputs::{BytesInput, HasMutatorBytes, Input},
};
use libafl_bolts::HasLen;
use lsp::encapsulate_request_content;
use path_segment::PathSegmentInput;
use serde::{Deserialize, Serialize};

pub mod lsp;
pub mod path_segment;

pub type FileContentInput = BytesInput;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathInput {
    pub segments: Vec<PathSegmentInput>,
}

impl PathInput {
    fn as_path_buf(&self) -> PathBuf {
        self.segments.iter().map(|it| it.as_path()).collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspRequestSequence {
    pub requests: Vec<lsp::LspRequest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspInput {
    pub requests: LspRequestSequence,
    pub source_directory: HashMap<PathInput, FileContentInput>,
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
        self.requests.requests.len() + self.source_directory.len()
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
        let did_open_requests = self.source_directory.keys().map(|source_file| {
            let mut path = EString::<fluent_uri::encoding::encoder::Path>::new();
            path.encode::<fluent_uri::encoding::encoder::Path>(
                workspace_dir
                    .join(source_file.as_path_buf())
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
        for (path, content) in self.source_directory.iter() {
            let path = source_dir.join(path.as_path_buf());
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content.bytes())?;
        }
        Ok(())
    }
}
