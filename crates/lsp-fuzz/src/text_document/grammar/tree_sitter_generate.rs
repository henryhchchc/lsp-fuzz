use lsp_fuzz_grammars::Language;
use lsp_fuzz_tree_sitter_grammar as tree_sitter_grammar;
use tree_sitter_grammar::{Symbol as TSSymbol, Terminal as TSTerminal};

use super::{CreationError, DerivationSequence, Grammar, Symbol, Terminal};

impl Grammar {
    /// Builds a [`Grammar`] from a Tree-sitter grammar JSON definition.
    ///
    /// # Errors
    ///
    /// Returns [`CreationError`] if the Tree-sitter grammar cannot be parsed,
    /// prepared, or converted into the internal grammar representation.
    pub fn from_tree_sitter_grammar_json(
        language: Language,
        grammar_json: &str,
    ) -> Result<Self, CreationError> {
        let prepared = tree_sitter_grammar::parse(grammar_json)
            .map_err(|error| CreationError::TreeSitter(error.into()))?;
        let derivation_rules = prepared
            .variables()
            .iter()
            .map(|variable| {
                let derivations = variable
                    .productions()
                    .iter()
                    .map(|production| {
                        let symbols = production.symbols().iter().map(convert_symbol).collect();
                        DerivationSequence::new(symbols)
                    })
                    .collect();
                (variable.name().to_owned(), derivations)
            })
            .collect();

        let start_symbol = prepared.start_symbol().to_owned();
        Ok(Self::new(language, start_symbol, derivation_rules))
    }
}

fn convert_symbol(symbol: &tree_sitter_grammar::Symbol) -> Symbol {
    match symbol {
        TSSymbol::NonTerminal(name) => Symbol::NonTerminal(name.clone()),
        TSSymbol::Terminal(terminal) => Symbol::Terminal(convert_terminal(terminal)),
        TSSymbol::End => Symbol::Eof,
    }
}

fn convert_terminal(terminal: &tree_sitter_grammar::Terminal) -> Terminal {
    match terminal {
        TSTerminal::Immediate(content) => Terminal::Immediate(content.clone()),
        TSTerminal::Named(name) => Terminal::Named(name.clone()),
        TSTerminal::Auxiliary(name) => Terminal::Auxiliary(name.clone()),
    }
}
