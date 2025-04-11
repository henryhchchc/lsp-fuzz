use std::{borrow::Cow, marker::PhantomData};

use derive_new::new as New;
use itertools::Itertools;
use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{HasLen, Named, rands::Rand};
use lsp_types::Uri;

use crate::lsp_input::LspInput;

use super::{
    GrammarBasedMutation, GrammarContextLookup, TextDocument, generation::GrammarContext,
    grammar::tree_sitter::TreeIter,
};

const MAX_DOCUMENT_SIZE: usize = libafl::state::DEFAULT_MAX_SIZE;

#[derive(Debug)]
pub struct ReplaceNodeMutation<'a, TS, NodeSel, NodeGen, State> {
    grammar_lookup: &'a GrammarContextLookup,
    name: Cow<'static, str>,
    node_selector: NodeSel,
    node_generator: NodeGen,
    _phantom: PhantomData<(TS, State)>,
}

impl<'a, TS, NF, NodeGen, State> ReplaceNodeMutation<'a, TS, NF, NodeGen, State> {
    pub fn new(
        grammar_lookup: &'a GrammarContextLookup,
        node_selector: NF,
        node_generator: NodeGen,
    ) -> Self {
        let name = Cow::Owned("ReplaceNode".to_owned());
        Self {
            grammar_lookup,
            name,
            node_selector,
            node_generator,
            _phantom: PhantomData,
        }
    }
}

impl<TS, NF, GEN, State> Named for ReplaceNodeMutation<'_, TS, NF, GEN, State> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &self.name
    }
}

pub trait TextDocumentSelector<State> {
    fn select_document<'i>(
        state: &mut State,
        input: &'i LspInput,
    ) -> Option<(Uri, &'i TextDocument)>;

    fn select_document_mut<'i>(
        state: &mut State,
        input: &'i mut LspInput,
    ) -> Option<(Uri, &'i mut TextDocument)>;
}

pub trait NodeSelector<State> {
    const NAME: &'static str;
    fn select_node<'t>(
        &self,
        doc: &'t mut TextDocument,
        grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<tree_sitter::Node<'t>>;
}

pub trait NodeGenerator<State> {
    const NAME: &'static str;
    fn generate_node(
        &self,
        node: tree_sitter::Node<'_>,
        grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<Vec<u8>>;
}

impl<State, TS, Sel, Gen> Mutator<LspInput, State> for ReplaceNodeMutation<'_, TS, Sel, Gen, State>
where
    TS: TextDocumentSelector<State>,
    Sel: NodeSelector<State>,
    Gen: NodeGenerator<State>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some((_, doc)) = TS::select_document_mut(state, input) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(grammar_ctx) = self.grammar_lookup.get(doc.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let doc_len = doc.len();
        let Some(selected_node) = self.node_selector.select_node(doc, grammar_ctx, state) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(replacement) =
            self.node_generator
                .generate_node(selected_node, grammar_ctx, state)
        else {
            return Ok(MutationResult::Skipped);
        };
        let node_len = selected_node.end_byte() - selected_node.start_byte();
        if doc_len - node_len + replacement.len() > MAX_DOCUMENT_SIZE {
            return Ok(MutationResult::Skipped);
        }
        let node_range = selected_node.range();
        doc.splice(node_range, replacement.to_vec());
        Ok(MutationResult::Mutated)
    }
}

pub mod text_document_selectors {
    use libafl::state::HasRand;
    use libafl_bolts::rands::Rand;
    use lsp_types::Uri;
    use std::option::Option;

    use crate::{lsp_input::LspInput, text_document::TextDocument};

    use super::TextDocumentSelector;

    #[derive(Debug)]
    pub struct RandomDoc;

    impl<State> TextDocumentSelector<State> for RandomDoc
    where
        State: HasRand,
    {
        fn select_document<'i>(
            state: &mut State,
            input: &'i LspInput,
        ) -> Option<(Uri, &'i TextDocument)> {
            let iter = input.workspace.iter_files().filter_map(|(path, doc)| {
                doc.as_source_file().map(|doc| {
                    (
                        format!("lsp-fuzz://{}", path.display()).parse().unwrap(),
                        doc,
                    )
                })
            });
            state.rand_mut().choose(iter)
        }

        fn select_document_mut<'i>(
            state: &mut State,
            input: &'i mut LspInput,
        ) -> Option<(Uri, &'i mut TextDocument)> {
            let iter = input.workspace.iter_files_mut().filter_map(|(path, doc)| {
                doc.as_source_file_mut().map(|doc| {
                    (
                        format!("lsp-fuzz://{}", path.display()).parse().unwrap(),
                        doc,
                    )
                })
            });
            state.rand_mut().choose(iter)
        }
    }
}

pub mod node_filters {
    use derive_new::new as New;
    use libafl::state::HasRand;
    use libafl_bolts::rands::Rand;

    use crate::text_document::{
        GrammarBasedMutation, GrammarContext, TextDocument,
        grammar::tree_sitter::{CapturesIterator, TreeIter},
    };

    use super::NodeSelector;

    #[derive(Debug, Clone, Copy, New)]
    pub struct NodesThat<Predicate> {
        predicate: Predicate,
    }

    impl<State, Predicate> NodeSelector<State> for NodesThat<Predicate>
    where
        State: HasRand,
        Predicate: Fn(&tree_sitter::Node<'_>) -> bool,
    {
        const NAME: &'static str = "NotesThat<Pred>";

        fn select_node<'t>(
            &self,
            doc: &'t mut TextDocument,
            _grammar_context: &GrammarContext,
            state: &mut State,
        ) -> Option<tree_sitter::Node<'t>> {
            let parse_tree = doc.parse_tree();
            let candidate_nodes = parse_tree.iter().filter(&self.predicate);
            state.rand_mut().choose(candidate_nodes)
        }
    }

    #[derive(Debug, Clone, New)]
    pub struct HighlightedNodes {
        capture_group_name: String,
    }

    impl<State> NodeSelector<State> for HighlightedNodes
    where
        State: HasRand,
    {
        const NAME: &'static str = "Highlighted";

        fn select_node<'t>(
            &self,
            doc: &'t mut TextDocument,
            _grammar_context: &GrammarContext,
            state: &mut State,
        ) -> Option<tree_sitter::Node<'t>> {
            let captured_nodes = CapturesIterator::new(doc, &self.capture_group_name)?;
            state.rand_mut().choose(captured_nodes)
        }
    }
}

pub mod node_generators {

    use std::{option::Option, vec::Vec};

    use libafl::{HasMetadata, state::HasRand};
    use libafl_bolts::rands::Rand;

    use crate::text_document::generation::{GrammarContext, NamedNodeGenerator, RuleUsageSteer};

    use super::NodeGenerator;

    #[derive(Debug)]
    pub struct EmptyNode;

    impl<State> NodeGenerator<State> for EmptyNode {
        const NAME: &'static str = "AnEmptyNode";
        fn generate_node(
            &self,
            _node: tree_sitter::Node<'_>,
            _grammar_context: &GrammarContext,
            _state: &mut State,
        ) -> Option<Vec<u8>> {
            Some(Vec::new())
        }
    }

    #[derive(Debug)]
    pub struct ChooseFromDerivations;

    impl<State> NodeGenerator<State> for ChooseFromDerivations
    where
        State: HasRand,
    {
        const NAME: &'static str = "RandomDerivation";
        fn generate_node(
            &self,
            node: tree_sitter::Node<'_>,
            grammar_context: &GrammarContext,
            state: &mut State,
        ) -> Option<Vec<u8>> {
            let fragments = grammar_context.node_fragments(node.kind());
            state.rand_mut().choose(fragments).map(|it| it.to_vec())
        }
    }

    #[derive(Debug)]
    pub struct ExpandGrammar;

    impl<State> NodeGenerator<State> for ExpandGrammar
    where
        State: HasRand + HasMetadata,
    {
        const NAME: &'static str = "RandomGeneration";
        fn generate_node(
            &self,
            node: tree_sitter::Node<'_>,
            grammar_context: &GrammarContext,
            state: &mut State,
        ) -> Option<Vec<u8>> {
            let selection_strategy = RuleUsageSteer;
            let generator = NamedNodeGenerator::new(grammar_context, selection_strategy);
            let fragment = generator.generate(node.kind(), state).ok()?;
            Some(fragment)
        }
    }
}

#[derive(Debug, New)]
pub struct DropUncoveredArea<TS> {
    _doc_selector: PhantomData<TS>,
}

impl<TS> Named for DropUncoveredArea<TS> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("DropUncoveredArea");
        &NAME
    }
}

impl<State, TS> Mutator<LspInput, State> for DropUncoveredArea<TS>
where
    TS: TextDocumentSelector<State>,
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some((_path, doc)) = TS::select_document_mut(state, input) else {
            return Ok(MutationResult::Skipped);
        };
        let covered_areas = doc
            .parse_tree()
            .iter()
            .filter(|it| it.child_count() > 0)
            .map(|it| it.range())
            .sorted_by_key(|it| it.start_byte)
            .tuple_windows()
            .filter(|(prev, curr)| prev.end_byte < curr.start_byte);

        let Some((prev, curr)) = state.rand_mut().choose(covered_areas) else {
            return Ok(MutationResult::Skipped);
        };

        doc.edit(|content| {
            let remove_range = prev.end_byte..curr.start_byte;
            let _ = content.drain(remove_range);
            tree_sitter::InputEdit {
                start_byte: prev.end_byte,
                old_end_byte: curr.start_byte,
                new_end_byte: prev.end_byte,
                start_position: prev.end_point,
                old_end_position: curr.start_point,
                new_end_position: prev.end_point,
            }
        });

        Ok(MutationResult::Mutated)
    }
}
