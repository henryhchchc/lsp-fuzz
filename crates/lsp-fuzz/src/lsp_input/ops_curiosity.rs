use std::{
    borrow::Cow,
    collections::VecDeque,
    hash::{Hash, Hasher},
    option::Option,
};

use ahash::{AHasher, HashSet, HashSetExt};
use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use fastbloom::BloomFilter;
use libafl::{
    HasMetadata, SerdeAny,
    feedbacks::{Feedback, StateInitializer},
    state::HasCorpus,
};
use libafl_bolts::Named;
use serde::{Deserialize, Serialize};

use super::{GrammarBasedMutation, Language};
use crate::{
    lsp::{ClientToServerMessage, code_context::CodeContextRef},
    lsp_input::LspInput,
    text_document::TextDocument,
    utils::AflContext,
};

#[derive(Debug, New)]
pub struct CuriosityFeedback;

impl Named for CuriosityFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("CuriosityFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for CuriosityFeedback
where
    State: HasMetadata,
{
    fn init_state(&mut self, state: &mut State) -> Result<(), libafl::Error> {
        state.add_metadata(ObservedOpsBehaviors::default());
        Ok(())
    }
}

impl<EM, OBS, State> Feedback<EM, LspInput, OBS, State> for CuriosityFeedback
where
    State: HasMetadata + HasCorpus<LspInput>,
{
    fn is_interesting(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        input: &LspInput,
        _observers: &OBS,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        let observed_ops: &mut ObservedOpsBehaviors = state
            .metadata_mut()
            .expect("We inserted that at the beginning");
        let behavior_data = analyze_behavior_data(input).afl_context("Analyzing behavior data")?;
        let is_interesting = behavior_data
            .into_iter()
            // The merge operation must be on the left hand side to make sure it is always performed.
            .fold(false, move |acc, ref it| observed_ops.merge(it) || acc);
        Ok(is_interesting)
    }
}

fn analyze_behavior_data(input: &LspInput) -> Result<HashSet<OpsBehaviorData>, libafl::Error> {
    let mut data = HashSet::new();
    for op in input.messages.iter() {
        if let Some(doc) = op.document()
            && let Some(doc) = input.get_text_document(&doc.uri)
        {
            if let Some(position) = op.position()
                && let Some(ops_data) = digest_ops_data(op, doc, position)
            {
                data.insert(ops_data);
            }
            if let Some(range) = op.range()
                && let Some(ops_data) = digest_range_data(op, doc, range)
            {
                data.insert(ops_data);
            }
        }
    }

    Ok(data)
}

fn digest_ops_data(
    op: &ClientToServerMessage,
    doc: &TextDocument,
    position: &lsp_types::Position,
) -> Option<OpsBehaviorData> {
    let ts_point = tree_sitter::Point {
        row: position.line as usize,
        column: position.character as usize,
    };
    if let Some(node) = doc
        .parse_tree()
        .root_node()
        .descendant_for_point_range(ts_point, ts_point)
        && node.child_count() == 0
    {
        let mut hasher = AHasher::default();
        hash_node_path(node, &mut hasher);
        let syntactic_signature = hasher.finish();
        let language = doc.language();
        let ops_method = op.method();
        Some(OpsBehaviorData {
            language,
            syntactic_signature,
            ops_method,
        })
    } else {
        None
    }
}

fn digest_range_data(
    op: &ClientToServerMessage,
    doc: &TextDocument,
    range: &lsp_types::Range,
) -> Option<OpsBehaviorData> {
    let start = tree_sitter::Point {
        row: range.start.line as usize,
        column: range.start.character as usize,
    };
    let end = tree_sitter::Point {
        row: range.end.line as usize,
        column: range.end.character as usize,
    };
    if let Some(node) = doc
        .parse_tree()
        .root_node()
        .descendant_for_point_range(start, end)
    {
        let mut hasher = AHasher::default();
        hash_node_path(node, &mut hasher);
        let mut curaor = node.walk();
        for child in node.children(&mut curaor) {
            if child.range().start_point <= start && child.range().end_point <= end {
                child.grammar_id().hash(&mut hasher);
            }
        }

        let syntactic_signature = hasher.finish();
        let language = doc.language();
        let ops_method = op.method();
        Some(OpsBehaviorData {
            language,
            syntactic_signature,
            ops_method,
        })
    } else {
        None
    }
}

fn hash_node_path<H: Hasher>(node: tree_sitter::Node<'_>, hasher: &mut H) {
    let mut next_visit = Some(node);
    while let Some(node) = next_visit {
        node.grammar_id().hash(hasher);
        next_visit = node.parent();
    }
}

pub fn hash_paths(parse_tree: &tree_sitter::Tree, max_depth: usize) -> Option<HashSet<u64>> {
    let mut hashes = HashSet::new();

    let mut queue = VecDeque::new();
    let hasher = AHasher::default();
    let root_node = parse_tree.root_node();
    queue.push_back((root_node, hasher, 0));
    while let Some((node, mut hasher, depth)) = queue.pop_front() {
        if depth >= max_depth {
            return None;
        }
        hash_node(&mut hasher, node);
        if node.child_count() == 0 {
            hashes.insert(hasher.finish());
        } else {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                queue.push_back((child, hasher.clone(), depth + 1));
            }
        }
    }
    Some(hashes)
}

fn hash_node(hasher: &mut AHasher, node: tree_sitter::Node<'_>) {
    hasher.write_u16(node.grammar_id());
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct OpsBehaviorData {
    language: Language,
    syntactic_signature: u64,
    ops_method: &'static str,
}

#[derive(Debug, Serialize, Deserialize, Deref, DerefMut, SerdeAny)]
pub struct ObservedOpsBehaviors {
    inner: BloomFilter,
}

impl Default for ObservedOpsBehaviors {
    fn default() -> Self {
        Self {
            inner: BloomFilter::with_false_pos(0.0001).expected_items(1_000_000),
        }
    }
}

impl ObservedOpsBehaviors {
    pub fn merge(&mut self, new_data: &OpsBehaviorData) -> bool {
        !self.inner.insert(new_data)
    }
}

#[cfg(test)]
mod tests {
    use lsp_fuzz_grammars::Language;

    #[test]
    fn node_hashing() {
        let rust_code = r#"
        fn main() {
            println!("Hello, world!");
        }
        "#;

        let mut parser = Language::Rust.tree_sitter_parser();
        let parse_tree = parser.parse(rust_code, None).unwrap();
        let hashes = super::hash_paths(&parse_tree, 10).unwrap();

        assert_eq!(14, hashes.len());
    }
}
