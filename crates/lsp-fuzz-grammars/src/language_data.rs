use crate::language::LanguageInfo;

macro_rules! include_grammar_json {
    ($name: literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/grammar/",
            $name,
            ".json"
        ))
    };
}

/// The C language information
pub const C: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("c"),
    extensions: &["c", "cc", "h"],
    highlight_query: tree_sitter_c::HIGHLIGHT_QUERY,
    lsp_language_id: "c",
    ts_language_fn: tree_sitter_c::LANGUAGE,
};

/// The C++ language information
pub const CPP: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("cpp"),
    extensions: &["cpp", "cxx", "hpp"],
    highlight_query: tree_sitter_cpp::HIGHLIGHT_QUERY,
    lsp_language_id: "cpp",
    ts_language_fn: tree_sitter_cpp::LANGUAGE,
};

/// The JavaScript language information
pub const JAVASCRIPT: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("javascript"),
    extensions: &["js"],
    highlight_query: tree_sitter_javascript::HIGHLIGHT_QUERY,
    lsp_language_id: "javascript",
    ts_language_fn: tree_sitter_javascript::LANGUAGE,
};

/// The Ruby language information
pub const RUBY: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("ruby"),
    extensions: &["rb"],
    highlight_query: tree_sitter_ruby::HIGHLIGHTS_QUERY,
    lsp_language_id: "ruby",
    ts_language_fn: tree_sitter_ruby::LANGUAGE,
};

/// The Rust language information
pub const RUST: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("rust"),
    extensions: &["rs"],
    highlight_query: tree_sitter_rust::HIGHLIGHTS_QUERY,
    lsp_language_id: "rust",
    ts_language_fn: tree_sitter_rust::LANGUAGE,
};

/// The Toml language information
pub const TOML: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("toml"),
    extensions: &["toml"],
    highlight_query: tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
    lsp_language_id: "toml",
    ts_language_fn: tree_sitter_toml_ng::LANGUAGE,
};

/// The LaTeX language information
pub const LATEX: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("latex"),
    extensions: &["tex", "dtx"],
    highlight_query: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/res/highlights/latex.scm"
    )),
    lsp_language_id: "latex",
    ts_language_fn: tree_sitter_latex::LANGUAGE,
};

/// The BibTeX language information
pub const BIBTEX: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("bibtex"),
    extensions: &["bib"],
    highlight_query: tree_sitter_bibtex::HIGHLIGHTS_QUERY,
    lsp_language_id: "bibtex",
    ts_language_fn: tree_sitter_bibtex::LANGUAGE,
};

/// The Verilog language information
pub const VERILOG: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("verilog"),
    extensions: &["v", "sv", "svh"],
    highlight_query: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/res/highlights/verilog.scm"
    )),
    lsp_language_id: "verilog",
    ts_language_fn: tree_sitter_verilog::LANGUAGE,
};

/// The Solidity language information
pub const SOLIDITY: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("solidity"),
    extensions: &["sol"],
    highlight_query: tree_sitter_solidity::HIGHLIGHT_QUERY,
    lsp_language_id: "solidity",
    ts_language_fn: tree_sitter_solidity::LANGUAGE,
};

/// The MLIR language information
pub const MLIR: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("mlir"),
    extensions: &["mlir"],
    highlight_query: tree_sitter_mlir::HIGHLIGHTS_QUERY,
    lsp_language_id: "mlir",
    ts_language_fn: tree_sitter_mlir::LANGUAGE,
};

/// The QML language information
pub const QML: LanguageInfo = LanguageInfo {
    grammar_json: include_grammar_json!("qml"),
    extensions: &["qml"],
    highlight_query: tree_sitter_qmljs::HIGHLIGHTS_QUERY,
    lsp_language_id: "qml",
    ts_language_fn: tree_sitter_qmljs::LANGUAGE,
};
