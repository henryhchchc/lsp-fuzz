use std::collections::HashMap;

use libafl::{corpus::CorpusId, inputs::Input};
use libafl_bolts::HasLen;
use serde::{Deserialize, Serialize};

use crate::utf8::Utf8Input;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemEntryInput<F> {
    File(F),
    Directory(HashMap<Utf8Input, FileSystemEntryInput<F>>),
}

impl<S: Input> Input for FileSystemEntryInput<S> {
    fn generate_name(&self, id: Option<CorpusId>) -> String {
        format!(
            "fs_input_id_{}",
            id.map(|it| it.to_string()).unwrap_or("unknown".to_owned())
        )
    }
}

impl<F: HasLen> HasLen for FileSystemEntryInput<F> {
    fn len(&self) -> usize {
        match self {
            FileSystemEntryInput::File(f) => f.len(),
            FileSystemEntryInput::Directory(entries) => entries
                .iter()
                .map(|(name, content)| name.len() + content.len())
                .sum(),
        }
    }
}

impl<F> FileSystemEntryInput<F> {
    /// Returns if the entry is a file.
    pub const fn is_file(&self) -> bool {
        matches!(self, FileSystemEntryInput::File(_))
    }

    /// Returns if the entry is a directory.
    pub const fn is_directory(&self) -> bool {
        matches!(self, FileSystemEntryInput::Directory(_))
    }

    pub fn is_leave(&self) -> bool {
        match self {
            FileSystemEntryInput::File(_) => true,
            FileSystemEntryInput::Directory(entries) => entries.is_empty(),
        }
    }
}
