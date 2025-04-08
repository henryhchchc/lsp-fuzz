use core::fmt;
use derive_new::new as New;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Formatter},
    ops::Range,
};

use anyhow::bail;
pub mod tree;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use libafl_bolts::rands::Rand;
use serde::{Deserialize, Serialize};
pub mod fragment_extraction;

use crate::stolen::tree_sitter_generate;

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
    pub const fn derivation_rules(&self) -> &IndexMap<String, IndexSet<DerivationSequence>> {
        &self.derivation_rules
    }

    pub fn from_tree_sitter_grammar_json(
        language: Language,
        grammar_json: &str,
    ) -> Result<Self, CreationError> {
        let input_grammar =
            tree_sitter_generate::parse_grammar(grammar_json).map_err(CreationError::TreeSitter)?;
        let (syntax_grammar, lexical_grammar, aliases) =
            tree_sitter_generate::prepare_grammar(&input_grammar)
                .map_err(CreationError::TreeSitter)?;
        Self::from_tree_sitter_grammar(language, syntax_grammar, lexical_grammar, aliases)
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

#[derive(Debug, Serialize, Deserialize, derive_more::Constructor)]
pub struct DerivationFragments {
    code: Vec<u8>,
    fragments: HashMap<Cow<'static, str>, Vec<Range<usize>>>,
}

#[derive(Debug, Default)]
pub struct FragmentsIter<'a> {
    code: &'a [u8],
    ranges: <&'a Vec<Range<usize>> as IntoIterator>::IntoIter,
}

impl DerivationFragments {
    pub fn get(&self, node_kind: &str) -> Option<FragmentsIter<'_>> {
        let ranges = self.fragments.get(node_kind)?;
        Some(FragmentsIter {
            code: &self.code,
            ranges: ranges.iter(),
        })
    }
}

impl<'a> Iterator for FragmentsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.ranges.next().cloned().map(|range| &self.code[range])
    }
}

impl ExactSizeIterator for FragmentsIter<'_> {
    fn len(&self) -> usize {
        self.ranges.len()
    }
}

#[derive(Debug, Serialize, Deserialize, derive_more::Constructor)]
pub struct GrammarContext {
    grammar: Grammar,
    node_fragments: DerivationFragments,
}

impl GrammarContext {
    pub fn create_parser(&self) -> tree_sitter::Parser {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.grammar.language.ts_language())
            .expect("Invalid tree-sitter language");
        parser
    }

    pub fn parse_source_code(
        &self,
        source_code: impl AsRef<[u8]>,
    ) -> Result<tree_sitter::Tree, tree_sitter::LanguageError> {
        let mut parser = self.create_parser();
        let tree = parser.parse(source_code, None).expect("Guaranteed by API");
        Ok(tree)
    }

    pub fn language(&self) -> Language {
        self.grammar.language
    }

    pub fn node_fragments(&self, node_kind: &str) -> FragmentsIter<'_> {
        self.node_fragments.get(node_kind).unwrap_or_default()
    }

    pub fn start_symbol(&self) -> &str {
        &self.grammar.start_symbol
    }

    pub fn start_symbol_fragments(
        &self,
    ) -> Result<impl ExactSizeIterator<Item = &[u8]>, DerivationError> {
        self.node_fragments
            .get(&self.grammar.start_symbol)
            .ok_or(DerivationError::InvalidGrammar)
    }

    pub fn generate_node<R: Rand>(
        &self,
        node_kind: &str,
        selection_strategy: &mut RuleSelectionStrategy<'_, R>,
        recursion_limit: Option<usize>,
    ) -> Result<Vec<u8>, DerivationError> {
        if recursion_limit.is_some_and(|it| it == 0) {
            return self.generate_from_fragments(node_kind, |choices| {
                selection_strategy.select_fragment(choices)
            });
        }
        if let Some(derivation_rules) = self.grammar.derivation_rules().get(node_kind) {
            let rule = selection_strategy
                .select_rule(derivation_rules)
                .ok_or(DerivationError::NoRuleAvailable)?;
            rule.into_iter()
                .map(|symbol| match symbol {
                    Symbol::NonTerminal(name) => {
                        let max_depth = recursion_limit.map(|it| it - 1);
                        self.generate_node(name, selection_strategy, max_depth)
                    }
                    Symbol::Terminal(term) => self.generate_terminal(term, |choices| {
                        selection_strategy.select_fragment(choices)
                    }),
                    Symbol::Eof => Ok(Vec::new()),
                })
                .flatten_ok()
                .collect::<Result<Vec<_>, _>>()
        } else {
            // We do not need to worry about unnamed terminals since they do not have a name.
            // They will never be passed in via `node_kind`.
            self.generate_from_fragments(node_kind, |choices| {
                selection_strategy.select_fragment(choices)
            })
        }
    }

    fn generate_from_fragments(
        &self,
        node_kind: &str,
        mut fragment_selector: impl FnMut(FragmentsIter<'_>) -> Option<&[u8]>,
    ) -> Result<Vec<u8>, DerivationError> {
        let fragments = self.node_fragments(node_kind);

        let fragment = fragment_selector(fragments).ok_or(DerivationError::NoFragmentAvailable)?;
        Ok(fragment.to_vec())
    }

    fn generate_terminal(
        &self,
        term: &Terminal,
        fragment_selector: impl FnMut(FragmentsIter<'_>) -> Option<&[u8]>,
    ) -> Result<Vec<u8>, DerivationError> {
        match term {
            Terminal::Immediate(content) => Ok(content.to_vec()),
            Terminal::Named(name) | Terminal::Auxiliary(name) => self
                .generate_from_fragments(name, fragment_selector)
                .map(|it| it.to_vec()),
        }
    }
}

#[derive(Debug, New)]
pub struct RuleSelectionStrategy<'a, R> {
    rand: &'a mut R,
}

impl<R> RuleSelectionStrategy<'_, R>
where
    R: Rand,
{
    fn select_fragment<'a>(&mut self, fragments: FragmentsIter<'a>) -> Option<&'a [u8]> {
        self.rand.choose(fragments)
    }

    fn select_rule<'a>(
        &mut self,
        rules: &'a IndexSet<DerivationSequence>,
    ) -> Option<&'a DerivationSequence> {
        self.rand.choose(rules)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DerivationError {
    #[error("The depth limit has been reached")]
    DepthLimitReached,
    #[error("The grammar is invalid")]
    InvalidGrammar,
    #[error("No rule available for the given node kind")]
    NoRuleAvailable,
    #[error("No fragment available for the given node kind")]
    NoFragmentAvailable,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn load_derivation_grammar_c() {
        let grammar =
            Grammar::from_tree_sitter_grammar_json(Language::C, Language::C.grammar_json())
                .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }

    #[test]
    fn load_derivation_grammar_cpp() {
        let grammar = Grammar::from_tree_sitter_grammar_json(
            Language::CPlusPlus,
            Language::CPlusPlus.grammar_json(),
        )
        .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }

    #[test]
    fn load_derivation_grammar_javascript() {
        let grammar = Grammar::from_tree_sitter_grammar_json(
            Language::JavaScript,
            Language::JavaScript.grammar_json(),
        )
        .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }

    #[test]
    fn load_derivation_grammar_rust() {
        let grammar =
            Grammar::from_tree_sitter_grammar_json(Language::Rust, Language::Rust.grammar_json())
                .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }

    #[test]
    fn load_derivation_grammar_toml() {
        let grammar =
            Grammar::from_tree_sitter_grammar_json(Language::Toml, Language::Toml.grammar_json())
                .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }

    #[test]
    fn load_derivation_grammar_latex() {
        let grammar =
            Grammar::from_tree_sitter_grammar_json(Language::LaTeX, Language::LaTeX.grammar_json())
                .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }

    #[test]
    fn load_derivation_grammar_bibtex() {
        let grammar = Grammar::from_tree_sitter_grammar_json(
            Language::BibTeX,
            Language::BibTeX.grammar_json(),
        )
        .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }

    #[test]
    fn load_derivation_grammar_solidity() {
        let grammar = Grammar::from_tree_sitter_grammar_json(
            Language::Solidity,
            Language::Solidity.grammar_json(),
        )
        .unwrap();
        eprintln!("{}", grammar);
        grammar.validate().unwrap();
    }
}
