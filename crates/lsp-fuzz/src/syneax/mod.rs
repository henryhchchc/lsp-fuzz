use crate::stolen::tree_sitter_generate::{parse_input_grammar, produce_syntax_grammar};

pub use crate::stolen::tree_sitter_generate::*;

pub const C_GRAMMAR_JSON: &str = include_str!("grammars/c.json");

pub fn load_syntax(grammar_json: &str) -> SyntaxGrammar {
    let input_grammar = parse_input_grammar(grammar_json).unwrap();
    produce_syntax_grammar(&input_grammar).unwrap()
}
