use std::{collections::BTreeSet, sync::OnceLock};

use tree_sitter_language::LanguageFn;

use super::Language;
use crate::language_data;

pub(super) struct LanguageInfo {
    pub extensions: &'static [&'static str],
    pub highlight_query: &'static str,
    pub grammar_json: &'static str,
    pub lsp_language_id: &'static str,
    pub ts_language_fn: LanguageFn,
}

impl Language {
    #[inline]
    const fn info(&self) -> LanguageInfo {
        match self {
            Language::C => language_data::C,
            Language::CPlusPlus => language_data::CPP,
            Language::JavaScript => language_data::JAVASCRIPT,
            Language::Ruby => language_data::RUBY,
            Language::Rust => language_data::RUST,
            Language::Toml => language_data::TOML,
            Language::LaTeX => language_data::LATEX,
            Language::BibTeX => language_data::BIBTEX,
            Language::Verilog => language_data::VERILOG,
            Language::Solidity => language_data::SOLIDITY,
            Language::MLIR => language_data::MLIR,
            Language::QML => language_data::QML,
        }
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
    pub fn ts_highlight_query(&self) -> &'static tree_sitter::Query {
        const VARIANT_COUNT: usize = 12;
        // Use `variant_count` when stabilized.
        // static QUERIES: [OnceLock<tree_sitter::Query>; variant_count::<Language>()] =
        //     [const { OnceLock::new() }; variant_count::<Language>()];
        static QUERIES: [OnceLock<tree_sitter::Query>; VARIANT_COUNT] =
            [const { OnceLock::new() }; VARIANT_COUNT];

        let query_idx = (*self as u8) as usize;
        QUERIES[query_idx].get_or_init(|| {
            let query_src = self.info().highlight_query;
            tree_sitter::Query::new(&self.ts_language(), query_src)
                .expect("The query provided by tree-sitter should be correct")
        })
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
