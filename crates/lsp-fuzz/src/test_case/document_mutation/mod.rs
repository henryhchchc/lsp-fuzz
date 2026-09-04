use std::{borrow::Cow, marker::PhantomData};

use derive_new::new as New;
use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{HasLen, Named};
use lsp_types::Uri;

use crate::{
    test_case::LspInput,
    text_document::{
        GrammarBasedMutation, TextDocument,
        generation::GrammarContextLookup,
        mutations::{
            MAX_DOCUMENT_SIZE,
            core::{NodeContentMutator, NodeGenerator, NodeSelector},
        },
    },
};

pub mod selectors;

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

type ReplaceNodeInRandomDoc<'a, NodeSel, NodeGen> =
    ReplaceNodeMutation<'a, selectors::RandomDoc, NodeSel, NodeGen>;
type NodeMutationInRandomDoc<'a, Mut, NodeSel> =
    NodeContentMutation<'a, Mut, selectors::RandomDoc, NodeSel>;

#[must_use]
pub fn text_document_mutations<'g, State>(
    grammar_lookup: &'g GrammarContextLookup,
    generators_config: &super::message_generation::GeneratorsConfig,
) -> impl libafl::mutators::MutatorsTuple<LspInput, State>
+ libafl_bolts::tuples::NamedTuple
+ use<'g, State>
where
    State: libafl::HasMetadata + libafl::state::HasMaxSize + HasRand,
{
    use crate::{
        mutators::WithProbability,
        text_document::mutations::{
            NodeTruncation,
            node_filters::{HighlightedNodes, NodesThat},
            node_generators::{ChooseFromDerivations, EmptyNode, ExpandGrammar, MismatchedNode},
        },
    };
    use libafl_bolts::tuples::Merge;
    use tuple_list::tuple_list;

    let any_node = NodesThat::new(|_: &tree_sitter::Node<'_>| true);
    let terminal_node = NodesThat::new(|it: &tree_sitter::Node<'_>| it.child_count() == 0);
    let remove_comment = ReplaceNodeInRandomDoc::new(
        grammar_lookup,
        HighlightedNodes::new("comment".to_owned()),
        EmptyNode,
    );
    let correct_code_mutations = tuple_list![
        ReplaceNodeInRandomDoc::new(grammar_lookup, any_node, ChooseFromDerivations),
        ReplaceNodeInRandomDoc::new(grammar_lookup, any_node, ChooseFromDerivations),
        ReplaceNodeInRandomDoc::new(grammar_lookup, any_node, ExpandGrammar),
        ReplaceNodeInRandomDoc::new(grammar_lookup, any_node, ExpandGrammar),
        ReplaceNodeInRandomDoc::new(grammar_lookup, any_node, ExpandGrammar),
        ReplaceNodeInRandomDoc::new(grammar_lookup, any_node, ExpandGrammar),
        remove_comment.clone(),
        remove_comment.clone(),
        remove_comment,
    ];
    let incorrect_code_mutations = {
        let recover_from_error = ReplaceNodeInRandomDoc::new(
            grammar_lookup,
            NodesThat::new(|it: &tree_sitter::Node<'_>| it.is_error()),
            ChooseFromDerivations,
        );
        let produce_missing_node = ReplaceNodeInRandomDoc::new(
            grammar_lookup,
            NodesThat::new(|it: &tree_sitter::Node<'_>| it.is_missing()),
            ChooseFromDerivations,
        );
        let generate_mismatched =
            ReplaceNodeInRandomDoc::new(grammar_lookup, any_node, MismatchedNode);
        let terminal_truncation =
            NodeMutationInRandomDoc::new(NodeTruncation, grammar_lookup, terminal_node);
        let drop_terminal = ReplaceNodeInRandomDoc::new(grammar_lookup, terminal_node, EmptyNode);

        tuple_list![
            recover_from_error,
            produce_missing_node,
            generate_mismatched.with_probability(generators_config.invalid_input.code_frequency),
            terminal_truncation.with_probability(generators_config.invalid_input.code_frequency),
            drop_terminal
                .clone()
                .with_probability(generators_config.invalid_input.code_frequency),
            drop_terminal.with_probability(generators_config.invalid_input.code_frequency),
        ]
    };
    correct_code_mutations.merge(incorrect_code_mutations)
}

#[derive(Debug)]
pub struct ReplaceNodeMutation<'a, TS, NodeSel, NodeGen> {
    grammar_lookup: &'a GrammarContextLookup,
    name: Cow<'static, str>,
    node_selector: NodeSel,
    node_generator: NodeGen,
    _phantom: PhantomData<TS>,
}

impl<TS, NodeSel: Clone, NodeGen: Clone> Clone for ReplaceNodeMutation<'_, TS, NodeSel, NodeGen> {
    fn clone(&self) -> Self {
        Self {
            grammar_lookup: self.grammar_lookup,
            name: self.name.clone(),
            node_selector: self.node_selector.clone(),
            node_generator: self.node_generator.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, TS, NodeSel, NodeGen> ReplaceNodeMutation<'a, TS, NodeSel, NodeGen> {
    pub fn new(
        grammar_lookup: &'a GrammarContextLookup,
        node_selector: NodeSel,
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

impl<TS, NodeSel, NodeGen> Named for ReplaceNodeMutation<'_, TS, NodeSel, NodeGen> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &self.name
    }
}

impl<State, DocSel, Sel, Gen> Mutator<LspInput, State> for ReplaceNodeMutation<'_, DocSel, Sel, Gen>
where
    DocSel: TextDocumentSelector<State>,
    Sel: NodeSelector<State>,
    Gen: NodeGenerator<State>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some((ref doc_uri, doc)) = DocSel::select_document_mut(state, input) else {
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
        let input_edit = doc.splice(node_range, replacement);
        input.messages.calibrate(doc_uri, input_edit);
        Ok(MutationResult::Mutated)
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, New)]
pub struct NodeContentMutation<'a, Mut, TS, NodeSel> {
    mutator: Mut,
    grammar_lookup: &'a GrammarContextLookup,
    node_selector: NodeSel,
    _phantom: PhantomData<TS>,
}

impl<Mut, TS, NodeSel> Named for NodeContentMutation<'_, Mut, TS, NodeSel> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("TokenMutation");
        &NAME
    }
}

impl<State, DocSel, NodeSel, Mut> Mutator<LspInput, State>
    for NodeContentMutation<'_, Mut, DocSel, NodeSel>
where
    DocSel: TextDocumentSelector<State>,
    NodeSel: NodeSelector<State>,
    Mut: NodeContentMutator<State>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some((ref doc_uri, doc)) = DocSel::select_document_mut(state, input) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(grammar_ctx) = self.grammar_lookup.get(doc.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(selected_node) = self.node_selector.select_node(doc, grammar_ctx, state) else {
            return Ok(MutationResult::Skipped);
        };
        let byte_range = selected_node.byte_range();
        let node_range = selected_node.range();
        let mut node_content = doc
            .content()
            .get(byte_range)
            .expect("The node is within the document")
            .to_vec();
        let doc_len = doc.content().len();
        let node_len = node_content.len();
        self.mutator.mutate(&mut node_content, state);
        if doc_len - node_len + node_content.len() > MAX_DOCUMENT_SIZE {
            return Ok(MutationResult::Skipped);
        }
        let input_edit = doc.splice(node_range, node_content);
        input.messages.calibrate(doc_uri, input_edit);
        Ok(MutationResult::Mutated)
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}
