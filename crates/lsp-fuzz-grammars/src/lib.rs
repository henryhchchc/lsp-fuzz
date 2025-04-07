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
