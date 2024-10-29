use libafl::inputs::{HasTargetBytes, MutVecInput};
use libafl_bolts::{ownedref::OwnedSlice, HasLen};
use serde::{Deserialize, Serialize};

use crate::stolen::tree_sitter_generate::{parse_input_grammar, produce_syntax_grammar};

pub mod grammars;

pub fn load_syntax(grammar_json: &str) -> grammars::SyntaxGrammar {
    let input_grammar = parse_input_grammar(grammar_json).unwrap();
    produce_syntax_grammar(&input_grammar).unwrap()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    C,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocument {
    content: Vec<u8>,
    language: Language,
}

impl TextDocument {
    pub fn new(content: Vec<u8>, language: Language) -> Self {
        Self { content, language }
    }

    pub fn content_bytes_mut(&mut self) -> MutVecInput<'_> {
        MutVecInput::from(&mut self.content)
    }
}

impl HasTargetBytes for TextDocument {
    fn target_bytes(&self) -> OwnedSlice<'_, u8> {
        OwnedSlice::from(&self.content)
    }
}

impl HasLen for TextDocument {
    fn len(&self) -> usize {
        self.content.len()
    }
}
