use core::{NodeGenerator, NodeSelector, TextDocumentSelector};
use std::{borrow::Cow, marker::PhantomData};

use libafl::mutators::{MutationResult, Mutator};
use libafl_bolts::{HasLen, Named};

use super::{GrammarBasedMutation, GrammarContextLookup};
use crate::lsp_input::LspInput;

pub mod core;
pub mod node_filters;
pub mod node_generators;
pub mod text_document_selectors;

const MAX_DOCUMENT_SIZE: usize = libafl::state::DEFAULT_MAX_SIZE;

#[derive(Debug)]
pub struct ReplaceNodeMutation<'a, TS, NodeSel, NodeGen, State> {
    grammar_lookup: &'a GrammarContextLookup,
    name: Cow<'static, str>,
    node_selector: NodeSel,
    node_generator: NodeGen,
    _phantom: PhantomData<(TS, State)>,
}

impl<'a, TS, NodeSel, NodeGen, State> ReplaceNodeMutation<'a, TS, NodeSel, NodeGen, State> {
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

impl<TS, NodeSel, NodeGen, State> Named for ReplaceNodeMutation<'_, TS, NodeSel, NodeGen, State> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &self.name
    }
}

impl<State, DocSel, Sel, Gen> Mutator<LspInput, State>
    for ReplaceNodeMutation<'_, DocSel, Sel, Gen, State>
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
        let input_edit = doc.splice(node_range, replacement.to_vec());
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
