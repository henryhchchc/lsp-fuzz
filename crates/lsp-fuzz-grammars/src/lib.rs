use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};

pub mod data;
mod language;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Display, FromStr)]
#[non_exhaustive]
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
    ShaderLang,
    MLIR,
    QML,
}
