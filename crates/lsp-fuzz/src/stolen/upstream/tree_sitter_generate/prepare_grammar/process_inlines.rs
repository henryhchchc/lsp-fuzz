use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::stolen::upstream::tree_sitter_generate::{
    grammars::{InlinedProductionMap, LexicalGrammar, Production, ProductionStep, SyntaxGrammar},
    rules::SymbolType,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ProductionStepId {
    // A `None` value here means that the production itself was produced via inlining,
    // and is stored in the builder's `productions` vector, as opposed to being
    // stored in one of the grammar's variables.
    variable_index: Option<usize>,
    production_index: usize,
    step_index: usize,
}
