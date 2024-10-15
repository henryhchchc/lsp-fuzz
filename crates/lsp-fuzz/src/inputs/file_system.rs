use std::{collections::HashMap, fmt::Debug};

use libafl::inputs::Input;
use serde::{Deserialize, Serialize};

use crate::utf8::Utf8Input;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemEntryInput<S> {
    SourceFile(S),
    Directory(HashMap<Utf8Input, FileSystemEntryInput<S>>),
}

impl<S: Input> Input for FileSystemEntryInput<S> {
    fn generate_name(&self, id: Option<libafl::corpus::CorpusId>) -> String {
        format!(
            "input_id_{}",
            id.map(|it| it.to_string()).unwrap_or("unknown".to_owned())
        )
    }
}

