use std::collections::BTreeSet;

use super::Language;

impl Language {
    pub fn file_extensions<'a>(&self) -> BTreeSet<&'a str> {
        match self {
            Self::C => BTreeSet::from(["c", "cc", "h"]),
            Self::CPlusPlus => BTreeSet::from(["cpp", "cxx", "hpp"]),
            Self::Rust => BTreeSet::from(["rs"]),
        }
    }

    pub fn tree_sitter_parser(&self) -> tree_sitter::Parser {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.ts_language())
            .expect("Fail to initialize parser");
        parser
    }

    pub fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Self::C => tree_sitter::Language::new(tree_sitter_c::LANGUAGE),
            Self::CPlusPlus => tree_sitter::Language::new(tree_sitter_cpp::LANGUAGE),
            Self::Rust => tree_sitter::Language::new(tree_sitter_rust::LANGUAGE),
        }
    }

    pub const fn grammar_json<'a>(&self) -> &'a str {
        match self {
            Self::C => super::grammars::C_GRAMMAR_JSON,
            Self::CPlusPlus => super::grammars::CPP_GRAMMAR_JSON,
            Self::Rust => super::grammars::RUST_GRAMMAR_JSON,
        }
    }

    pub const fn lsp_language_id<'a>(&self) -> &'a str {
        match self {
            Self::C => "c",
            Self::CPlusPlus => "cpp",
            Self::Rust => "rust",
        }
    }
}
