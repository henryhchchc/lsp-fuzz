use tree_sitter_language::LanguageFn;

use crate::language_data;

use super::Language;
use std::collections::BTreeSet;

pub(super) struct LanguageInfo {
    pub extensions: &'static [&'static str],
    pub highlight_query: &'static str,
    pub grammar_json: &'static str,
    pub lsp_language_id: &'static str,
    pub ts_language_fn: LanguageFn,
}

// [TODO] Use `variant_count::<Language>()` when stablized.
const LANGUAGES_COUNT: usize = 12;

// # Important: The order of this array must be identical to the order of variants in the Language enum.
const LANGUAGES: [LanguageInfo; LANGUAGES_COUNT] = [
    language_data::C,
    language_data::CPP,
    language_data::JAVASCRIPT,
    language_data::RUBY,
    language_data::RUST,
    language_data::TOML,
    language_data::LATEX,
    language_data::BIBTEX,
    language_data::VERILOG,
    language_data::SOLIDITY,
    language_data::MLIR,
    language_data::QML,
];

impl Language {
    const fn info(&self) -> &'static LanguageInfo {
        let index = *self as u8 as usize;
        &LANGUAGES[index]
    }

    pub fn file_extensions<'a>(&self) -> BTreeSet<&'a str> {
        self.info().extensions.iter().copied().collect()
    }

    pub fn tree_sitter_parser(&self) -> tree_sitter::Parser {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.ts_language())
            .expect("Fail to initialize parser");
        parser
    }

    /// Query for tree-sitter syntax highlighting
    ///
    /// See the following two links for common highlight groups
    /// - [Neovim](https://neovim.io/doc/user/treesitter.html#treesitter-highlight-groups)
    /// - [Zed](https://zed.dev/docs/extensions/languages#syntax-highlighting)
    pub fn ts_highlight_query(&self) -> tree_sitter::Query {
        let query_src = self.info().highlight_query;
        tree_sitter::Query::new(&self.ts_language(), query_src)
            .expect("The query provided by tree-sitter should be correct")
    }

    pub fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::new(self.info().ts_language_fn)
    }

    pub const fn grammar_json<'a>(&self) -> &'a str {
        self.info().grammar_json
    }

    /// The language identifier used by the Language Server Protocol
    /// See https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentItem
    pub const fn lsp_language_id<'a>(&self) -> &'a str {
        self.info().lsp_language_id
    }
}
