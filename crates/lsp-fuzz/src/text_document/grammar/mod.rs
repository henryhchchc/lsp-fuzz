use anyhow::bail;
use core::fmt;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod fragment_extraction;
pub mod tree_sitter;

use super::Language;
/// Represents a terminal symbol in a grammar.
///
/// A terminal symbol is a basic building block in a grammar that cannot be broken
/// down further. These represent the actual tokens or literals in the language.
#[derive(Debug, Hash, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
pub enum Terminal {
    /// An immediate terminal with literal bytes.
    #[display("\"{}\"", String::from_utf8_lossy(_0).escape_default())]
    Immediate(Vec<u8>),

    /// A named terminal that refers to a specific token type.
    #[display("[{_0}]")]
    Named(String),

    /// An auxiliary terminal used for special cases or helper tokens.
    #[display("({_0})")]
    Auxiliary(String),
}

/// Represents a symbol in a grammar, which can be either a terminal or non-terminal.
///
/// Symbols are the building blocks of production rules in a grammar. They can be
/// either terminals (which represent actual tokens) or non-terminals (which represent
/// abstractions that can be expanded using production rules).
#[derive(Debug, Hash, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
pub enum Symbol {
    /// A terminal symbol that cannot be expanded further
    Terminal(Terminal),

    /// A non-terminal symbol with a name, which can be expanded using production rules
    #[display("<{_0}>")]
    NonTerminal(String),

    /// The end of file symbol, marking the end of input
    #[display("<EOF>")]
    Eof,
}

/// Represents a sequence of symbols in a derivation rule.
///
/// A derivation sequence is the right-hand side of a production rule in a grammar.
/// It consists of a sequence of symbols (terminals and non-terminals) that a
/// non-terminal on the left-hand side can be expanded into.
#[derive(Debug, Hash, PartialEq, Eq, Serialize, Deserialize, derive_more::IntoIterator)]
pub struct DerivationSequence {
    /// The sequence of symbols that make up this derivation
    #[serde(flatten)]
    #[into_iterator(owned, ref, ref_mut)]
    symbols: Vec<Symbol>,
}

impl Display for DerivationSequence {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.symbols.is_empty() {
            write!(f, "ε")
        } else {
            write!(f, "{}", self.symbols.iter().format(" "))
        }
    }
}

impl DerivationSequence {
    pub fn new(symbols: Vec<Symbol>) -> Self {
        Self { symbols }
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }
}

/// Represents a formal grammar for a programming language.
///
/// A grammar consists of a language identifier, a start symbol, and a collection of
/// derivation rules that define how to generate valid programs in the language.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, derive_more::Constructor)]
pub struct Grammar {
    /// The programming language this grammar represents
    language: Language,
    /// The name of the starting non-terminal symbol for the grammar
    start_symbol: String,
    /// The production rules of the grammar, mapping non-terminal names to their possible derivation sequences
    derivation_rules: IndexMap<String, IndexSet<DerivationSequence>>,
}

impl Display for Grammar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Grammar for {}", self.language)?;
        writeln!(f, "Start symbol: <{}>", self.start_symbol)?;
        writeln!(f, "Production rules:")?;
        for (symbol, derivations) in &self.derivation_rules {
            writeln!(
                f,
                "<{}> ::=\n    {}\n",
                symbol,
                derivations.iter().format("\n  | ")
            )?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl Grammar {
    pub const fn language(&self) -> Language {
        self.language
    }

    pub fn start_symbol(&self) -> &str {
        self.start_symbol.as_str()
    }

    pub const fn derivation_rules(&self) -> &IndexMap<String, IndexSet<DerivationSequence>> {
        &self.derivation_rules
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        for symbol in self.derivation_rules.values().flatten().flatten() {
            match symbol {
                Symbol::NonTerminal(name) if !self.derivation_rules.contains_key(name) => {
                    bail!("Missing rule for non-terminal symbol: {}", name);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreationError {
    #[error("Error occurred in tree-sitter: {0}")]
    TreeSitter(anyhow::Error),
    #[error("The provided grammar is empty")]
    EmptyGrammar,
    #[error("The grammar is missing a rule")]
    MissingRule,
}

#[cfg(test)]
mod tests {

    use ::tree_sitter::QueryCursor;

    use crate::text_document::{TextDocument, grammar::tree_sitter::CapturesIterator};

    use super::*;

    #[test]
    fn load_all_derivation_grammars() {
        let languages = [
            Language::C,
            Language::CPlusPlus,
            Language::JavaScript,
            Language::Rust,
            Language::Toml,
            Language::LaTeX,
            Language::BibTeX,
            Language::Solidity,
        ];
        for language in languages {
            let grammar =
                Grammar::from_tree_sitter_grammar_json(language, language.grammar_json()).unwrap();
            eprintln!("{}", grammar);
            grammar
                .validate()
                .unwrap_or_else(|_| panic!("Fail to validate grammar for language: {language}"));
        }
    }

    #[test]
    fn capture_rust() {
        const RUST_CODE: &str = r#"
            // Hello
            fn main() {
                println!("Hello, world!");
            }
        "#;
        let doc = TextDocument::new(Language::Rust, RUST_CODE.as_bytes().to_vec());
        let mut cursor = QueryCursor::new();
        let mut capture_iter = CapturesIterator::new(&doc, "comment", &mut cursor).unwrap();
        let node = capture_iter.next().expect("There is one comment node");
        let text = &doc.content[node.byte_range()];
        assert_eq!(text, b"// Hello");

        let mut capture_iter = CapturesIterator::new(&doc, "keyword", &mut cursor).unwrap();
        let node = capture_iter.next().expect("There is one keyword node");
        let text = &doc.content[node.byte_range()];
        assert_eq!(text, b"fn");
    }
}
