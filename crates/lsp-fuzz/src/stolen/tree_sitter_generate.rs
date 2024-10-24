#![allow(unused_imports)]
use super::upstream::tree_sitter_generate::{
    self, parse_grammar::parse_grammar, prepare_grammar::prepare_grammar,
};

pub use tree_sitter_generate::{
    grammars::{
        ExternalToken, InputGrammar, LexicalGrammar, LexicalVariable, PrecedenceEntry, Production,
        SyntaxGrammar, SyntaxVariable, VariableType,
    },
    rules::Symbol,
};

pub(crate) fn parse_input_grammar(grammar_json: &str) -> Result<InputGrammar, anyhow::Error> {
    parse_grammar(grammar_json)
}

pub(crate) fn produce_syntax_grammar(
    input_grammar: &InputGrammar,
) -> Result<SyntaxGrammar, anyhow::Error> {
    let (syntax_grammar, _inlines, _simple_aliases) = prepare_grammar(input_grammar)?;
    Ok(syntax_grammar)
}
