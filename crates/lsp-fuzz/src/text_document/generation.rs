use indexmap::IndexSet;
use itertools::Itertools;
use libafl::{HasMetadata, state::HasRand};
use libafl_bolts::rands::Rand;
use lsp_fuzz_grammars::Language;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow, cmp::max, collections::HashMap, marker::PhantomData, num::NonZero, ops::Range,
};

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
    pub grammar: Grammar,
    pub node_fragments: DerivationFragments,
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
}

#[derive(Debug)]
pub struct NamedNodeGenerator<'a, State, Sel> {
    grammar_context: &'a GrammarContext,
    selection_strategy: Sel,
    _state: PhantomData<State>,
}

impl<'a, State, Sel> NamedNodeGenerator<'a, State, Sel> {
    pub const fn new(grammar_context: &'a GrammarContext, selection_strategy: Sel) -> Self {
        Self {
            grammar_context,
            selection_strategy,
            _state: PhantomData,
        }
    }
}

impl<State, Sel> NamedNodeGenerator<'_, State, Sel>
where
    Sel: RuleSelectionStrategy<State>,
{
    const DEFAULT_REDURSION_LIMIT: usize = 5;

    pub fn generate(&self, node_kind: &str, state: &mut State) -> Result<Vec<u8>, DerivationError> {
        self.generate_recursively(node_kind, state, Some(Self::DEFAULT_REDURSION_LIMIT))
    }

    fn generate_recursively(
        &self,
        node_kind: &str,
        state: &mut State,
        recursion_limit: Option<usize>,
    ) -> Result<Vec<u8>, DerivationError> {
        if recursion_limit.is_some_and(|it| it == 0) {
            return self.generate_from_fragments(node_kind, |choices| {
                self.selection_strategy.select_fragment(
                    state,
                    choices,
                    self.grammar_context.language(),
                )
            });
        }
        if let Some(derivation_rules) = self
            .grammar_context
            .grammar
            .derivation_rules()
            .get(node_kind)
        {
            let rule = self
                .selection_strategy
                .select_rule(
                    state,
                    node_kind,
                    derivation_rules,
                    self.grammar_context.language(),
                )
                .ok_or(DerivationError::NoRuleAvailable)?;
            rule.into_iter()
                .map(|symbol| match symbol {
                    Symbol::NonTerminal(name) => {
                        let recursion_limit = recursion_limit.map(|it| it - 1);
                        self.generate_recursively(name, state, recursion_limit)
                    }
                    Symbol::Terminal(term) => self.generate_terminal(term, |choices| {
                        self.selection_strategy.select_fragment(
                            state,
                            choices,
                            self.grammar_context.language(),
                        )
                    }),
                    Symbol::Eof => Ok(Vec::new()),
                })
                .flatten_ok()
                .collect::<Result<Vec<_>, _>>()
        } else {
            // We do not need to worry about unnamed terminals since they do not have a name.
            // They will never be passed in via `node_kind`.
            self.generate_from_fragments(node_kind, |choices| {
                self.selection_strategy.select_fragment(
                    state,
                    choices,
                    self.grammar_context.language(),
                )
            })
        }
    }

    fn generate_from_fragments(
        &self,
        node_kind: &str,
        mut fragment_selector: impl FnMut(FragmentsIter<'_>) -> Option<&[u8]>,
    ) -> Result<Vec<u8>, DerivationError> {
        let fragments = self.grammar_context.node_fragments(node_kind);

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

pub trait RuleSelectionStrategy<State> {
    fn select_fragment<'a>(
        &self,
        state: &mut State,
        fragments: FragmentsIter<'a>,
        language: Language,
    ) -> Option<&'a [u8]>;

    fn select_rule<'a>(
        &self,
        state: &mut State,
        node_kind: &str,
        rules: &'a IndexSet<DerivationSequence>,
        language: Language,
    ) -> Option<&'a DerivationSequence>;
}

#[derive(Debug)]
pub struct RandomRuleSelectionStrategy;

impl<State> RuleSelectionStrategy<State> for RandomRuleSelectionStrategy
where
    State: HasRand,
{
    fn select_fragment<'a>(
        &self,
        state: &mut State,
        fragments: FragmentsIter<'a>,
        _language: Language,
    ) -> Option<&'a [u8]> {
        state.rand_mut().choose(fragments)
    }

    fn select_rule<'a>(
        &self,
        state: &mut State,
        _node_kind: &str,
        rules: &'a IndexSet<DerivationSequence>,
        _language: Language,
    ) -> Option<&'a DerivationSequence> {
        state.rand_mut().choose(rules)
    }
}

#[derive(Debug)]
pub struct RuleUsageSteer;

#[derive(Debug, Serialize, Deserialize, Default, libafl_bolts::SerdeAny)]
pub struct RuleUsageStats {
    inner: ahash::HashMap<(Language, String), Vec<usize>>,
}

impl<State> RuleSelectionStrategy<State> for RuleUsageSteer
where
    State: HasRand + HasMetadata,
{
    fn select_fragment<'a>(
        &self,
        state: &mut State,
        fragments: FragmentsIter<'a>,
        _language: Language,
    ) -> Option<&'a [u8]> {
        state.rand_mut().choose(fragments)
    }

    fn select_rule<'a>(
        &self,
        state: &mut State,
        node_kind: &str,
        rules: &'a IndexSet<DerivationSequence>,
        language: Language,
    ) -> Option<&'a DerivationSequence> {
        let stats = state.metadata_or_insert_with::<RuleUsageStats>(Default::default);
        let stats = stats
            .inner
            .entry((language, node_kind.to_owned()))
            .or_insert(vec![0; rules.len()]);
        let usage_bounds = max(1, *stats.iter().max()?);
        let weights = stats.iter().map(|it| usage_bounds - it);
        let (range_lookup, max) = weights.enumerate().fold(
            (Vec::with_capacity(rules.len()), 0),
            |(mut map, start), (idx, weight)| {
                let end = start + weight;
                map.push((start..end, idx));
                (map, end)
            },
        );
        let chosen_point = state.rand_mut().below(NonZero::new(max)?);
        let chosen_idx = range_lookup
            .into_iter()
            .find_map(|(range, idx)| range.contains(&chosen_point).then_some(idx))?;
        // Have to reborrow which is a PITA.
        let stats = state
            .metadata_mut::<RuleUsageStats>()
            .expect("We inserted it before");
        let stats = stats
            .inner
            .get_mut(&(language, node_kind.to_owned()))
            .expect("We inserted it before");
        stats[chosen_idx] += 1;
        rules.get_index(chosen_idx)
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
