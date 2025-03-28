use super::{Language, grammars::GrammarJson};
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
    /// - https://neovim.io/doc/user/treesitter.html#treesitter-highlight-groups
    /// - https://zed.dev/docs/extensions/languages#syntax-highlighting
    pub fn ts_highlight_query(&self) -> tree_sitter::Query {
        let query_src = match self {
            Self::C => tree_sitter_c::HIGHLIGHT_QUERY,
            Self::CPlusPlus => tree_sitter_cpp::HIGHLIGHT_QUERY,
            Self::JavaScript => tree_sitter_javascript::HIGHLIGHT_QUERY,
            Self::Ruby => tree_sitter_ruby::HIGHLIGHTS_QUERY,
            Self::Rust => tree_sitter_rust::HIGHLIGHTS_QUERY,
            Self::Toml => tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            // Stolen from https://github.com/rzukic/zed-latex/blob/main/languages/latex/highlights.scm
            Self::LaTeX => include_str!("grammars/tree_sitter/highlights/latex.scm"),
            Self::BibTeX => tree_sitter_bibtex::HIGHLIGHTS_QUERY,
            // Stolen from https://github.com/someone13574/zed-verilog-extension/raw/refs/heads/main/languages/verilog/highlights.scm
            Self::Verilog => include_str!("grammars/tree_sitter/highlights/verilog.scm"),
            Self::Solidity => tree_sitter_solidity::HIGHLIGHT_QUERY,
            // [TODO] There is no query for slang avaliable yet.
            Self::ShaderLang => "",
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
        }
    }
}
