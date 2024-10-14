use std::{
    collections::HashMap,
    iter::once,
    path::{Path, PathBuf},
    str::FromStr,
};

use libafl::{
    corpus::CorpusId,
    inputs::{BytesInput, HasMutatorBytes, Input},
};
use libafl_bolts::HasLen;
use lsp::encapsulate_request_content;
use path_segment::PathSegmentInput;
use serde::{Deserialize, Serialize};
use tracing::info;

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
        let init_request = &lsp::LspRequest::Initialize(lsp_types::InitializeParams {
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
        let mut bytes = Vec::new();
        for (id, request) in self
            .requests
            .requests
            .iter()
            .chain(once(init_request))
            .enumerate()
        {
            bytes.extend_from_slice(&encapsulate_request_content(&request.as_json(id + 1)));
        }
        bytes
    }

    pub fn setup_source_dir(&self, source_dir: &Path) -> Result<(), std::io::Error> {
        for (path, content) in self.source_directory.iter() {
            let path = source_dir.join(path.as_path_buf());
            info!("Writing file: {:?}", path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content.bytes())?;
        }
        Ok(())
    }
}
