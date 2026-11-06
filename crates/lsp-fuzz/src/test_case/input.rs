use super::{HasWorkspace, WorkspaceEntry, messages::LspMessageSequence, session, uri};
use crate::{
    file_system::{FileSystemDirectory, FileSystemEntry},
    lsp,
    text_document::TextDocument,
};
use derive_new::new as New;
use libafl::{
    corpus::CorpusId,
    inputs::{Input, ToTargetBytesConverter},
};
use libafl_bolts::{HasLen, ownedref::OwnedSlice};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct LspInput {
    pub messages: LspMessageSequence,
    pub workspace: FileSystemDirectory<WorkspaceEntry>,
}

impl LspInput {
    pub const NAME_PREFIX: &str = "input_";
    pub const PROTOCOL_PREFIX: &str = "lsp-fuzz://";
    pub const PROROCOL_PREFIX: &str = Self::PROTOCOL_PREFIX;

    #[must_use]
    pub fn root_uri() -> Uri {
        uri::root_uri()
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
    /// # Panics
    ///
    /// Panics if `uri` does not start with [`LspInput::PROROCOL_PREFIX`].
    ///
    #[must_use]
    pub fn get_text_document(&self, uri: &lsp_types::Uri) -> Option<&TextDocument> {
        let path =
            uri::path_from_virtual_uri(uri).expect("The URI must start with the fuzzer URI scheme");
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
        let id_str = id.map_or_else(
            || {
                let mut hasher = DefaultHasher::new();
                self.hash(&mut hasher);
                format!("h_{}", hasher.finish())
            },
            |it| it.to_string(),
        );
        format!("{}{}", Self::NAME_PREFIX, id_str)
    }

    fn to_file<P>(&self, path: P) -> Result<(), libafl::Error>
    where
        P: AsRef<Path>,
    {
        let file = File::create(path)?;
        let buf_writer = BufWriter::new(file);
        ciborium::into_writer(self, buf_writer)
            .map_err(|e| libafl::Error::serialize(format!("{e:#?}")))
    }

    fn from_file<P>(path: P) -> Result<Self, libafl::Error>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let buf_reader = std::io::BufReader::new(file);
        ciborium::from_reader(buf_reader).map_err(|e| libafl::Error::serialize(format!("{e:#?}")))
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

impl<State> ToTargetBytesConverter<LspInput, State> for LspInputBytesConverter {
    fn convert_to_target_bytes<'a>(
        &mut self,
        _state: &mut State,
        input: &'a LspInput,
    ) -> OwnedSlice<'a, u8> {
        let workspace_dir = input.workspace_dir(&self.workspace_root);
        input.request_bytes(&workspace_dir).into()
    }
}

impl HasWorkspace for LspInput {
    fn workspace_hash(&self) -> u64 {
        let mut hasher = ahash::AHasher::default();
        self.workspace.hash(&mut hasher);
        hasher.finish()
    }

    fn setup_workspace(&self, workspace_root: &Path) -> Result<(), std::io::Error> {
        self.workspace.write_to_fs(workspace_root)
    }
}

impl LspInput {
    pub const WORKSPACE_DIR_PREFIX: &str = "lsp-fuzz-workspace_";

    /// Converts a localized `file://` workspace URI back into the virtual `lsp-fuzz://` form.
    ///
    /// # Panics
    ///
    /// Panics if the lifted URI cannot be parsed back into a valid [`Uri`].
    #[must_use]
    pub fn lift_uri(uri: &lsp_types::Uri) -> Cow<'_, lsp_types::Uri> {
        uri::lift_uri(uri)
    }

    /// Serializes the full LSP session into wire-format payload bytes.
    ///
    /// # Panics
    ///
    /// Panics if `workspace_dir` is not valid UTF-8.
    #[must_use]
    pub fn request_bytes(&self, workspace_dir: &Path) -> Vec<u8> {
        session::request_bytes(self, workspace_dir)
    }

    /// Expands the test case into JSON-RPC messages with virtual URIs localized to `workspace_dir`.
    ///
    /// # Panics
    ///
    /// Panics if `workspace_dir` is not valid UTF-8.
    pub fn localized_json_rpc_message_sequence(
        &self,
        workspace_dir: &Path,
    ) -> impl Iterator<Item = lsp::json_rpc::JsonRPCMessage> + use<'_> {
        session::localized_json_rpc_message_sequence(self, workspace_dir)
    }

    /// Expands the stored input into the complete LSP session message stream.
    ///
    /// # Panics
    ///
    /// Panics if a workspace source file path is not valid UTF-8 or if a generated virtual URI
    /// cannot be parsed as an [`Uri`].
    pub fn message_sequence(&self) -> impl Iterator<Item = lsp::LspMessage> + use<'_> {
        session::message_sequence(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pre_refactor_corpus_input() {
        let bytes = include_bytes!("fixtures/lsp_input.cbor");
        let input: LspInput = ciborium::from_reader(bytes.as_slice()).unwrap();
        assert!(input.messages.is_empty());
        assert_eq!(
            input.workspace.get("config.json"),
            Some(&FileSystemEntry::File(WorkspaceEntry::Skeleton(
                b"{}".to_vec()
            )))
        );
        let mut encoded = Vec::new();
        ciborium::into_writer(&input, &mut encoded).unwrap();
        assert_eq!(encoded, bytes);
    }
}
