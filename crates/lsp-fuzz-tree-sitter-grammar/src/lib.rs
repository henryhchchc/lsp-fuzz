//! The subset of Tree-sitter's generator needed to prepare derivation grammars.

#![allow(
    dead_code,
    reason = "the vendored preparation IR retains fields shared with omitted generator stages"
)]

mod bitvec;
mod grammars;
mod nfa;
mod parse_grammar;
mod prepare_grammar;
mod rules;
mod strpool;

use grammars::{LexicalVariable, ProductionStep, VariableType};
use rules::{Alias, SymbolType};
use strpool::StrPool;
use thiserror::Error;

pub use parse_grammar::ParseGrammarError;
pub use prepare_grammar::PrepareGrammarError;

/// A non-fatal diagnostic emitted while normalizing a grammar.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum Diagnostic {
    UnnecessaryConflicts(Vec<Vec<String>>),
    UnaryChoice { name: Option<String> },
    UnarySeq { name: Option<String> },
    EmptyStringMatch(String),
    UnsupportedRegexFlag { flag: char, pattern: String },
    SupertypeInlined { name: String },
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnnecessaryConflicts(conflicts) => {
                write!(f, "unnecessary conflicts: {conflicts:?}")
            }
            Self::UnaryChoice { name } => write!(
                f,
                "rule {} contains a unary choice",
                name.as_deref().unwrap_or("<anonymous>")
            ),
            Self::UnarySeq { name } => write!(
                f,
                "rule {} contains a unary sequence",
                name.as_deref().unwrap_or("<anonymous>")
            ),
            Self::EmptyStringMatch(rule) => {
                write!(f, "named extra rule `{rule}` matches the empty string")
            }
            Self::UnsupportedRegexFlag { flag, pattern } => {
                write!(f, "unsupported regex flag `{flag}` in pattern `{pattern}`")
            }
            Self::SupertypeInlined { name } => {
                write!(f, "rule `{name}` is both a supertype and inlined")
            }
        }
    }
}

/// A grammar represented as flattened derivation rules.
#[derive(Debug, PartialEq, Eq)]
pub struct Grammar {
    start_symbol: String,
    variables: Vec<Variable>,
    diagnostics: Vec<Diagnostic>,
}

impl Grammar {
    #[must_use]
    pub fn start_symbol(&self) -> &str {
        &self.start_symbol
    }

    #[must_use]
    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Variable {
    name: String,
    productions: Vec<Production>,
}

impl Variable {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn productions(&self) -> &[Production] {
        &self.productions
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Production(Vec<Symbol>);

impl Production {
    #[must_use]
    pub fn symbols(&self) -> &[Symbol] {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Symbol {
    NonTerminal(String),
    Terminal(Terminal),
    End,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Terminal {
    Immediate(Vec<u8>),
    Named(String),
    Auxiliary(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] ParseGrammarError),
    #[error(transparent)]
    Prepare(#[from] PrepareGrammarError),
    #[error("the grammar has no start symbol")]
    EmptyGrammar,
    #[error("a prepared production references a missing symbol")]
    MissingSymbol,
}

/// Parse and normalize a Tree-sitter `grammar.json` definition.
///
/// # Errors
///
/// Returns [`Error`] when the grammar cannot be parsed, prepared, or converted
/// to LSPFuzz's derivation representation.
pub fn parse(input: &str) -> Result<Grammar, Error> {
    let mut diagnostics = Vec::new();
    let input = parse_grammar::parse_grammar(input, &mut diagnostics)?;
    let prepared = prepare_grammar::prepare_grammar(input, &mut diagnostics)?;
    let syntax = &prepared.syntax_grammar;
    let lexical = &prepared.lexical_grammar;
    let aliases = &prepared.default_aliases;
    let strings = &prepared.str_pool;

    let start_symbol = syntax
        .variables
        .first()
        .map(|variable| strings.resolve(variable.name).to_owned())
        .ok_or(Error::EmptyGrammar)?;
    let variables = syntax
        .variables
        .iter()
        .enumerate()
        .map(|(index, variable)| {
            let productions = syntax
                .variable_prod_ids(index)
                .map(|id| syntax.production(id))
                .map(|production| {
                    production
                        .steps
                        .iter()
                        .map(|step| convert_symbol(*step, syntax, lexical, aliases, strings))
                        .collect::<Result<Vec<_>, _>>()
                        .map(Production)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Variable {
                name: strings.resolve(variable.name).to_owned(),
                productions,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(Grammar {
        start_symbol,
        variables,
        diagnostics,
    })
}

fn convert_symbol(
    step: ProductionStep,
    syntax: &grammars::SyntaxGrammar,
    lexical: &grammars::LexicalGrammar,
    aliases: &rules::AliasMap,
    strings: &StrPool,
) -> Result<Symbol, Error> {
    let symbol = step.symbol();
    let alias = step.alias().or_else(|| aliases.get(&symbol).copied());
    match symbol.kind {
        SymbolType::NonTerminal => syntax
            .variables
            .get(symbol.index as usize)
            .map(|variable| Symbol::NonTerminal(strings.resolve(variable.name).to_owned()))
            .ok_or(Error::MissingSymbol),
        SymbolType::Terminal => lexical
            .variables
            .get(symbol.index as usize)
            .map(|variable| Symbol::Terminal(convert_terminal(variable, alias, strings)))
            .ok_or(Error::MissingSymbol),
        SymbolType::External => syntax
            .external_tokens
            .get(symbol.index as usize)
            .map(|variable| {
                Symbol::Terminal(Terminal::Named(strings.resolve(variable.name).to_owned()))
            })
            .ok_or(Error::MissingSymbol),
        SymbolType::End | SymbolType::EndOfNonTerminalExtra => Ok(Symbol::End),
    }
}

fn convert_terminal(
    variable: &LexicalVariable,
    alias: Option<Alias>,
    strings: &StrPool,
) -> Terminal {
    if let Some(alias) = alias {
        let value = strings.resolve(alias.value);
        return if alias.is_named {
            Terminal::Named(value.to_owned())
        } else {
            Terminal::Immediate(value.as_bytes().to_vec())
        };
    }

    let name = strings.resolve(variable.name);
    match variable.kind {
        VariableType::Anonymous => Terminal::Immediate(name.as_bytes().to_vec()),
        VariableType::Auxiliary => Terminal::Auxiliary(name.to_owned()),
        VariableType::Named | VariableType::Hidden => Terminal::Named(name.to_owned()),
    }
}
