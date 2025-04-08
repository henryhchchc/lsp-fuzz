use indexmap::IndexSet;
use itertools::Itertools;
use libafl_bolts::rands::Rand;
use lsp_fuzz_grammars::Language;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, ops::Range};

use super::grammar::{DerivationSequence, Grammar, Symbol, Terminal};

#[derive(Debug, Serialize, Deserialize, libafl_bolts::SerdeAny)]
pub struct GrammarContextLookup {
    inner: HashMap<Language, GrammarContext>,
}

impl GrammarContextLookup {
    pub fn get(&self, language: Language) -> Option<&GrammarContext> {
        self.inner.get(&language)
    }

    pub fn iter(&self) -> impl Iterator<Item = &GrammarContext> {
        self.inner.values()
    }
}

impl FromIterator<GrammarContext> for GrammarContextLookup {
    fn from_iter<T: IntoIterator<Item = GrammarContext>>(iter: T) -> Self {
        let inner = iter.into_iter().map(|it| (it.language(), it)).collect();
        Self { inner }
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
            .set_language(&self.grammar.language().ts_language())
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
        self.grammar.language()
    }

    pub fn node_fragments(&self, node_kind: &str) -> FragmentsIter<'_> {
        self.node_fragments.get(node_kind).unwrap_or_default()
    }

    pub fn start_symbol(&self) -> &str {
        self.grammar.start_symbol()
    }

    pub fn generate_node<S: RuleSelectionStrategy>(
        &self,
        node_kind: &str,
        selection_strategy: &mut S,
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

pub trait RuleSelectionStrategy {
    fn select_fragment<'a>(&mut self, fragments: FragmentsIter<'a>) -> Option<&'a [u8]>;

    fn select_rule<'a>(
        &mut self,
        rules: &'a IndexSet<DerivationSequence>,
    ) -> Option<&'a DerivationSequence>;
}

#[derive(Debug)]
pub struct RandomRuleSelectionStrategy<'a, R> {
    rand: &'a mut R,
}

impl<'a, R> RandomRuleSelectionStrategy<'a, R> {
    pub fn new(rand: &'a mut R) -> Self {
        Self { rand }
    }
}

impl<R> RuleSelectionStrategy for RandomRuleSelectionStrategy<'_, R>
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
