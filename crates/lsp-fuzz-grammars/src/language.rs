use crate::data::{GrammarHighLights, GrammarJson};

use super::Language;
use std::collections::BTreeSet;

impl Language {
    pub fn file_extensions<'a>(&self) -> BTreeSet<&'a str> {
        let extensions: &[&str] = match self {
            Self::C => &["c", "cc", "h"],
            Self::CPlusPlus => &["cpp", "cxx", "hpp"],
            Self::JavaScript => &["js"],
            Self::Ruby => &["rb"],
            Self::Rust => &["rs"],
            Self::Toml => &["toml"],
            Self::LaTeX => &["tex", "dtx"],
            Self::BibTeX => &["bib"],
            Self::Verilog => &["v", "sv", "svh"],
            Self::Solidity => &["sol"],
            Self::ShaderLang => &["slang"],
            Self::MLIR => &["mlir"],
            Self::QML => &["qml"],
        };
        extensions.iter().copied().collect()
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
        let query_src = match self {
            Self::C => GrammarHighLights::C,
            Self::CPlusPlus => GrammarHighLights::CPP,
            Self::JavaScript => GrammarHighLights::JAVASCRIPT,
            Self::Ruby => GrammarHighLights::RUBY,
            Self::Rust => GrammarHighLights::RUST,
            Self::Toml => GrammarHighLights::TOML,
            Self::LaTeX => GrammarHighLights::LATEX,
            Self::BibTeX => GrammarHighLights::BIBTEX,
            Self::Verilog => GrammarHighLights::VERILOG,
            Self::Solidity => GrammarHighLights::SOLIDITY,
            Self::ShaderLang => GrammarHighLights::SHADERLANG,
            Self::MLIR => GrammarHighLights::MLIR,
            Self::QML => GrammarHighLights::QML,
        };
        tree_sitter::Query::new(&self.ts_language(), query_src)
            .expect("The query provided by tree-sitter should be correct")
    }

    pub fn ts_language(&self) -> tree_sitter::Language {
        let lang_fn = match self {
            Self::C => tree_sitter_c::LANGUAGE,
            Self::CPlusPlus => tree_sitter_cpp::LANGUAGE,
            Self::JavaScript => tree_sitter_javascript::LANGUAGE,
            Self::Ruby => tree_sitter_ruby::LANGUAGE,
            Self::Rust => tree_sitter_rust::LANGUAGE,
            Self::Toml => tree_sitter_toml_ng::LANGUAGE,
            Self::LaTeX => tree_sitter_latex::LANGUAGE,
            Self::BibTeX => tree_sitter_bibtex::LANGUAGE,
            Self::Verilog => tree_sitter_verilog::LANGUAGE,
            Self::Solidity => tree_sitter_solidity::LANGUAGE,
            Self::ShaderLang => tree_sitter_slang::LANGUAGE,
            Self::MLIR => tree_sitter_mlir::LANGUAGE,
            Self::QML => tree_sitter_qmljs::LANGUAGE,
        };
        tree_sitter::Language::new(lang_fn)
    }

    pub const fn grammar_json<'a>(&self) -> &'a str {
        match self {
            Self::C => GrammarJson::C,
            Self::CPlusPlus => GrammarJson::CPP,
            Self::JavaScript => GrammarJson::JAVASCRIPT,
            Self::Ruby => GrammarJson::RUBY,
            Self::Rust => GrammarJson::RUST,
            Self::Toml => GrammarJson::TOML,
            Self::LaTeX => GrammarJson::LATEX,
            Self::BibTeX => GrammarJson::BIBTEX,
            Self::Verilog => GrammarJson::VERILOG,
            Self::Solidity => GrammarJson::SOLIDITY,
            Self::ShaderLang => GrammarJson::SLANG,
            Self::MLIR => GrammarJson::MLIR,
            Self::QML => GrammarJson::QML,
        }
    }

    /// The language identifier used by the Language Server Protocol
    /// See https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentItem
    pub const fn lsp_language_id<'a>(&self) -> &'a str {
        match self {
            Self::C => "c",
            Self::CPlusPlus => "cpp",
            Self::JavaScript => "javascript",
            Self::Ruby => "ruby",
            Self::Rust => "rust",
            Self::Toml => "toml",
            Self::LaTeX => "latex",
            Self::BibTeX => "bibtex",
            Self::Verilog => "verilog",
            Self::Solidity => "solidity",
            Self::ShaderLang => "slang",
            Self::MLIR => "mlir",
            Self::QML => "qml",
        }
    }
}
