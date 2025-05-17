use core::{NodeContentMutator, NodeGenerator, NodeSelector, TextDocumentSelector};
use std::{borrow::Cow, marker::PhantomData};

use derive_new::new as New;
use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{HasLen, Named, rands::Rand};

use super::{GrammarBasedMutation, GrammarContextLookup};
use crate::lsp_input::LspInput;

pub mod core;
pub mod node_filters;
pub mod node_generators;
pub mod text_document_selectors;

pub const MAX_DOCUMENT_SIZE: usize = 100_000;

#[derive(Debug)]
pub struct ReplaceNodeMutation<'a, TS, NodeSel, NodeGen> {
    grammar_lookup: &'a GrammarContextLookup,
    name: Cow<'static, str>,
    node_selector: NodeSel,
    node_generator: NodeGen,
    _phantom: PhantomData<TS>,
}

impl<'a, TS, NodeSel: Clone, NodeGen: Clone> Clone
    for ReplaceNodeMutation<'a, TS, NodeSel, NodeGen>
{
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
            .content
            .get(byte_range)
            .expect("The node is within the document")
            .to_vec();
        let doc_len = doc.content.len();
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

#[derive(Debug, Copy, Clone)]
pub struct NodeTruncation;

impl<State> NodeContentMutator<State> for NodeTruncation
where
    State: HasRand,
{
    fn mutate(&self, content: &mut Vec<u8>, state: &mut State) {
        let string = String::from_utf8_lossy(content).into_owned();
        let truncate_position = state.rand_mut().below_or_zero(string.chars().count());
        let string: String = string.chars().take(truncate_position).collect();
        *content = string.into_bytes();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NodeUTF8Mutation;

impl<State> NodeContentMutator<State> for NodeUTF8Mutation
where
    State: HasRand,
{
    fn mutate(&self, content: &mut Vec<u8>, state: &mut State) {
        let rand = state.rand_mut();
        let mut string = String::from_utf8_lossy(content).into_owned();
        let Some((idx, picked)) = rand.choose(string.char_indices()) else {
            return;
        };
        let new_char = rand
            .choose(char::MIN..char::MAX)
            .expect("There must be a char inside this range.");
        string.replace_range(idx..idx + picked.len_utf8(), &new_char.to_string());
        *content = string.into_bytes()
    }
}
