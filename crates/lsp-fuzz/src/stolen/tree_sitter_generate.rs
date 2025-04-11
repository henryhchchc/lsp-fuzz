//! APIs exposed from the [`tree_sitter_generate`](https://github.com/tree-sitter/tree-sitter/tree/master/cli/generate) project.

use indexmap::IndexSet;
use itertools::Itertools;
use lsp_fuzz_grammars::Language;

use super::upstream::tree_sitter_generate::{
    grammars::{
        LexicalGrammar, LexicalVariable, ProductionStep, SyntaxGrammar, SyntaxVariable,
        VariableType,
    },
    parse_grammar::parse_grammar,
    prepare_grammar::prepare_grammar,
    rules::{Alias, AliasMap, SymbolType},
};
use crate::text_document::grammar::{CreationError, DerivationSequence, Grammar, Symbol, Terminal};

impl Grammar {
    pub fn from_tree_sitter_grammar_json(
        language: Language,
        grammar_json: &str,
    ) -> Result<Self, CreationError> {
        let input_grammar = parse_grammar(grammar_json).map_err(CreationError::TreeSitter)?;
        let (syntax_grammar, lexical_grammar, aliases) =
            prepare_grammar(&input_grammar).map_err(CreationError::TreeSitter)?;
        Self::from_tree_sitter_grammar(language, syntax_grammar, lexical_grammar, aliases)
    }

    pub(crate) fn from_tree_sitter_grammar(
        language: lsp_fuzz_grammars::Language,
        syntax_grammar: SyntaxGrammar,
        lexical_grammar: LexicalGrammar,
        alias_map: AliasMap,
    ) -> Result<Self, CreationError> {
        let start_symbol = syntax_grammar
            .variables
            .first()
            .ok_or(CreationError::EmptyGrammar)?
            .name
            .clone();
        let derivation_rules = syntax_grammar
            .variables
            .iter()
            .map(|syntax_variable| {
                convert_rule(
                    syntax_variable,
                    &syntax_grammar,
                    &lexical_grammar,
                    &alias_map,
                )
            })
            .try_collect()?;
        Ok(Self::new(language, start_symbol, derivation_rules))
    }
}

fn convert_terminal(rule: &LexicalVariable, alias: Option<&Alias>) -> Terminal {
    match rule.kind {
        VariableType::Anonymous => {
            // assert!(alias.is_none(), "{:?} -> {:?}", rule, alias);
            Terminal::Immediate(rule.name.clone().into_bytes())
        }
        VariableType::Auxiliary => {
            if let Some(alias) = alias {
                if alias.is_named {
                    Terminal::Named(alias.value.clone())
                } else {
                    Terminal::Immediate(alias.value.clone().into_bytes())
                }
            } else {
                Terminal::Auxiliary(rule.name.clone())
            }
        }
        VariableType::Named => {
            if let Some(alias) = alias {
                if alias.is_named {
                    Terminal::Named(alias.value.clone())
                } else {
                    Terminal::Immediate(alias.value.clone().into_bytes())
                }
            } else {
                Terminal::Named(rule.name.clone())
            }
        }
        VariableType::Hidden => {
            // eprintln!("Rule: {:?}\nAlias:{:?}", rule, alias);
            // todo!("Figure out what hidden terminals are")
            Terminal::Named(rule.name.clone())
        }
    }
}

fn convert_symbol(
    step: &ProductionStep,
    syntax_grammar: &SyntaxGrammar,
    lexical_grammar: &LexicalGrammar,
    alias_map: &AliasMap,
) -> Result<Symbol, CreationError> {
    let rule_idx = step.symbol.index;
    let alias = alias_map.get(&step.symbol);
    match step.symbol.kind {
        SymbolType::NonTerminal => {
            let rule = syntax_grammar
                .variables
                .get(rule_idx)
                .ok_or(CreationError::MissingRule)?;
            Ok(Symbol::NonTerminal(rule.name.clone()))
        }
        SymbolType::Terminal => {
            let rule = lexical_grammar
                .variables
                .get(rule_idx)
                .ok_or(CreationError::MissingRule)?;
            let terminal = convert_terminal(rule, alias);
            Ok(Symbol::Terminal(terminal))
        }
        SymbolType::External => {
            let rule = syntax_grammar
                .external_tokens
                .get(rule_idx)
                .ok_or(CreationError::MissingRule)?;
            let terminal = Terminal::Named(rule.name.clone());
            Ok(Symbol::Terminal(terminal))
        }
        SymbolType::End | SymbolType::EndOfNonTerminalExtra => Ok(Symbol::Eof),
    }
}

fn convert_rule(
    syntax_variable: &SyntaxVariable,
    syntax_grammar: &SyntaxGrammar,
    lexical_grammar: &LexicalGrammar,
    alias_map: &AliasMap,
) -> Result<(String, IndexSet<DerivationSequence>), CreationError> {
    let derivations = syntax_variable
        .productions
        .iter()
        .map(|production_rule| {
            let symbols = production_rule
                .steps
                .iter()
                .map(|step| convert_symbol(step, syntax_grammar, lexical_grammar, alias_map))
                .try_collect()?;
            Ok(DerivationSequence::new(symbols))
        })
        .try_collect()?;
    Ok((syntax_variable.name.clone(), derivations))
}
