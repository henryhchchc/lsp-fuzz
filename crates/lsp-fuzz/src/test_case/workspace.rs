use super::LspInput;
use crate::text_document::TextDocument;
use libafl::inputs::HasTargetBytes;
use libafl_bolts::{HasLen, ownedref::OwnedSlice};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Provides the virtual workspace needed to execute a fuzz test case.
pub trait HasWorkspace {
    fn workspace_hash(&self) -> u64;

    /// Returns the deterministic on-disk directory for this workspace.
    fn workspace_dir(&self, root: &Path) -> PathBuf {
        root.join(format!(
            "{}{hash}",
            LspInput::WORKSPACE_DIR_PREFIX,
            hash = self.workspace_hash()
        ))
    }

    /// Creates the on-disk workspace layout rooted at `workspace_root`.
    ///
    /// # Errors
    ///
    /// Returns any I/O error encountered while creating directories or files
    /// for the workspace.
    fn setup_workspace(&self, workspace_root: &Path) -> Result<(), std::io::Error>;
}

/// An entry in the LSP server workspace
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceEntry {
    /// A text document that will be sent to the LSP server
    ///
    /// A `textDocument/didOpen` will be issued for this entry after LSP server initialization.
    SourceFile(TextDocument),

    /// A skeleton file within the workspace
    ///
    /// The file will not be sent to the LSP server after initialization.
    /// It is only written to the workspace directory for LSP servers that needs it.
    /// (e.g., `package.json`, `Cargo.toml`).
    Skeleton(Vec<u8>),
}

impl WorkspaceEntry {
    #[must_use]
    pub const fn as_source_file(&self) -> Option<&TextDocument> {
        if let WorkspaceEntry::SourceFile(doc) = self {
            Some(doc)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_source_file_mut(&mut self) -> Option<&mut TextDocument> {
        if let WorkspaceEntry::SourceFile(doc) = self {
            Some(doc)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_skeleton(&self) -> Option<&[u8]> {
        if let WorkspaceEntry::Skeleton(bytes) = self {
            Some(bytes.as_slice())
        } else {
            None
        }
    }

    #[must_use]
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
