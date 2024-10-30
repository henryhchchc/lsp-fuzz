//! APIs exposed from the [`tree_sitter_generate`](https://github.com/tree-sitter/tree-sitter/tree/master/cli/generate) project.

use crate::text_document::grammars::{
    CreationError, Derivation, DerivationGrammar, Symbol, Terminal,
};
use itertools::Itertools;

use super::upstream::tree_sitter_generate::grammars::{
    LexicalVariable, ProductionStep, SyntaxVariable,
};

pub(crate) use super::upstream::tree_sitter_generate::{
    grammars::{LexicalGrammar, SyntaxGrammar, VariableType},
    parse_grammar::parse_grammar,
    prepare_grammar::prepare_grammar,
    rules::{AliasMap, SymbolType},
};

impl DerivationGrammar {
    fn convert_terminal(rule: &LexicalVariable) -> Terminal {
        match rule.kind {
            VariableType::Anonymous => Terminal::Literal(rule.name.clone().into_bytes()),
            VariableType::Named => Terminal::Named(rule.name.clone()),
            VariableType::Auxiliary => Terminal::Auxillary(rule.name.clone()),
            VariableType::Hidden => {
                todo!("Figure out what hidden terminals are")
            }
        }
    }

    fn convert_symbol(
        step: &ProductionStep,
        syntax_grammar: &SyntaxGrammar,
        lexical_grammar: &LexicalGrammar,
    ) -> Result<Symbol, CreationError> {
        let rule_idx = step.symbol.index;
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
                let terminal = Self::convert_terminal(rule);
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
    ) -> Result<(String, Vec<Derivation>), CreationError> {
        let derivations = syntax_variable
            .productions
            .iter()
            .map(|production_rule| {
                let symbols = production_rule
                    .steps
                    .iter()
                    .map(|step| Self::convert_symbol(step, syntax_grammar, lexical_grammar))
                    .try_collect()?;
                Ok(Derivation::new(symbols))
            })
            .try_collect()?;
        Ok((syntax_variable.name.clone(), derivations))
    }

    pub(crate) fn from_tree_sitter_grammar(
        language: crate::text_document::Language,
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
        let mut derivation_rules = syntax_grammar
            .variables
            .iter()
            .map(|syntax_variable| {
                Self::convert_rule(syntax_variable, &syntax_grammar, &lexical_grammar)
            })
            .try_collect()?;
        todo!("Implement aliasing");
        Ok(Self::new(language, start_symbol, derivation_rules))
    }
}
