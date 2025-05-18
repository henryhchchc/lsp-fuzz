#![feature(variant_count)]

use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};

mod language;
mod language_data;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Display, FromStr)]
#[non_exhaustive]
#[repr(u8)]
pub enum Language {
    C,
    CPlusPlus,
    JavaScript,
    Ruby,
    Rust,
    Toml,
    LaTeX,
    BibTeX,
    Verilog,
    Solidity,
    MLIR,
    QML,
}

/// Well-known highlight capture names.
///
/// This list is based on the well-known highlight capture names used by popular editors and IDEs.
///
/// - [Neovim](https://neovim.io/doc/user/treesitter.html#treesitter-highlight-groups)
/// - [Zed](https://zed.dev/docs/extensions/languages#syntax-highlighting)
pub const WELL_KNOWN_HIGHLIGHT_CAPTURE_NAMES: [&str; 9] = [
    "string",
    "number",
    "keyword",
    "operator",
    "identifier",
    "type",
    "function",
    "constant",
    "variable",
];
