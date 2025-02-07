use std::collections::BTreeSet;

use super::{grammars::GrammarJson, Language};

impl Language {
    pub fn file_extensions<'a>(&self) -> BTreeSet<&'a str> {
        match self {
            Self::C => BTreeSet::from(["c", "cc", "h"]),
            Self::CPlusPlus => BTreeSet::from(["cpp", "cxx", "hpp"]),
            Self::JavaScript => BTreeSet::from(["js"]),
            Self::Rust => BTreeSet::from(["rs"]),
            Self::Toml => BTreeSet::from(["toml"]),
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
        let lang_fn = match self {
            Self::C => tree_sitter_c::LANGUAGE,
            Self::CPlusPlus => tree_sitter_cpp::LANGUAGE,
            Self::JavaScript => tree_sitter_javascript::LANGUAGE,
            Self::Rust => tree_sitter_rust::LANGUAGE,
            Self::Toml => tree_sitter_toml_ng::LANGUAGE,
        };
        tree_sitter::Language::new(lang_fn)
    }

    pub const fn grammar_json<'a>(&self) -> &'a str {
        match self {
            Self::C => GrammarJson::C,
            Self::CPlusPlus => GrammarJson::CPP,
            Self::JavaScript => GrammarJson::JAVASCRIPT,
            Self::Rust => GrammarJson::RUST,
            Self::Toml => GrammarJson::TOML,
        }
    }

    pub const fn lsp_language_id<'a>(&self) -> &'a str {
        match self {
            Self::C => "c",
            Self::CPlusPlus => "cpp",
            Self::JavaScript => "javascript",
            Self::Rust => "rust",
            Self::Toml => "toml",
        }
    }
}
