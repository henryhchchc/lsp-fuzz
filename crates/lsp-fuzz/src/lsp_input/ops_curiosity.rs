use std::{
    borrow::Cow,
    collections::VecDeque,
    hash::{Hash, Hasher},
    iter,
    option::Option,
};

use ahash::{AHasher, HashSet, HashSetExt};
use derive_more::derive::{Deref, DerefMut};
use fastbloom::BloomFilter;
use libafl::{
    HasMetadata, SerdeAny,
    feedbacks::{Feedback, StateInitializer},
    observers::{Observer, ObserversTuple},
    state::HasCorpus,
};
use libafl_bolts::{
    Named,
    tuples::{Handle, Handled, MatchNameRef},
};
use serde::{Deserialize, Serialize};

use super::{GrammarBasedMutation, Language};
use crate::{
    lsp::{ClientToServerMessage, code_context::CodeContextRef},
    lsp_input::LspInput,
    text_document::TextDocument,
    utils::AflContext,
};

#[derive(Debug)]
pub struct CuriosityFeedback {
    observer_handle: Handle<OpsBehaviorObserver>,
}

impl CuriosityFeedback {
    pub fn new(observer: &OpsBehaviorObserver) -> Self {
        Self {
            observer_handle: observer.handle(),
        }
    }
}

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
    OBS: ObserversTuple<LspInput, State>,
{
    fn is_interesting(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        _input: &LspInput,
        observers: &OBS,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        let metadata: &mut ObservedOpsBehaviors = state
            .metadata_mut()
            .expect("We inserted that at the beginning");
        let observer = observers
            .get(&self.observer_handle)
            .afl_context("OpsBehaviorObserver not found")?;

        let behavior_data = observer
            .observed_behavior()
            .afl_context("Observer did not observe any behavior.")?;
        let is_interesting = behavior_data.iter().any(|it| !metadata.contains(it));
        Ok(is_interesting)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        observers: &OBS,
        _testcase: &mut libafl::corpus::Testcase<LspInput>,
    ) -> Result<(), libafl::Error> {
        let metadata: &mut ObservedOpsBehaviors = state
            .metadata_mut()
            .expect("We inserted that at the beginning");
        let observer = observers
            .get(&self.observer_handle)
            .afl_context("OpsBehaviorObserver not found")?;
        let behavior_data = observer
            .observed_behavior()
            .afl_context("Observer did not observe any behavior.")?;
        behavior_data.iter().for_each(|it| {
            metadata.merge(it);
        });
        Ok(())
    }
}

fn analyze_behavior_data(
    input: &LspInput,
    max_syn_depth: usize,
) -> Result<HashSet<OpsBehaviorData>, libafl::Error> {
    let mut data = HashSet::new();
    for op in input.messages.iter() {
        if let Some(doc) = op.document()
            && let Some(doc) = input.get_text_document(&doc.uri)
        {
            if let Some(position) = op.position()
                && let Some(ops_data) = digest_ops_data(op, doc, position, max_syn_depth)
            {
                data.insert(ops_data);
            }
            if let Some(range) = op.range() {
                let ops_data = digest_range_data(op, doc, range, max_syn_depth);
                data.extend(ops_data);
            }
        }
    }

    Ok(data)
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct OpsBehaviorData {
    language: Language,
    node_kind: u16,
    ops_method: &'static str,
}

fn digest_ops_data(
    op: &ClientToServerMessage,
    doc: &TextDocument,
    position: &lsp_types::Position,
    _max_syn_depth: usize,
) -> Option<OpsBehaviorData> {
    let ts_point = tree_sitter::Point {
        row: position.line as usize,
        column: position.character as usize,
    };
    if let Some(node) = doc
        .parse_tree()
        .root_node()
        .descendant_for_point_range(ts_point, ts_point)
        && !node.has_error()
        && !node.is_missing()
    {
        let language = doc.language();
        let node_kind = node.kind_id();
        let ops_method = op.method();
        Some(OpsBehaviorData {
            language,
            node_kind,
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
    _max_syn_depth: usize,
) -> HashSet<OpsBehaviorData> {
    let mut data = HashSet::new();
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
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.range().start_point <= start
                && child.range().end_point <= end
                && !node.has_error()
                && !node.is_missing()
            {
                let language = doc.language();
                let ops_method = op.method();
                let node_kind = child.kind_id();
                data.insert(OpsBehaviorData {
                    language,
                    node_kind,
                    ops_method,
                });
            }
        }
    }
    data
}

pub fn hash_node_path<H: Hasher>(
    node: tree_sitter::Node<'_>,
    max_syn_depth: usize,
    hasher: &mut H,
) -> Option<()> {
    let syn_signature: Vec<_> = iter::successors(Some(node), |it| it.parent())
        .map(|node| node.grammar_id())
        .collect();
    if syn_signature.len() > max_syn_depth {
        None
    } else {
        syn_signature.into_iter().for_each(|it| it.hash(hasher));
        Some(())
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

#[derive(Debug, Serialize, Deserialize, Deref, DerefMut, SerdeAny)]
pub struct ObservedOpsBehaviors {
    inner: BloomFilter,
}

impl Default for ObservedOpsBehaviors {
    fn default() -> Self {
        let inner = BloomFilter::with_false_pos(0.0001).expected_items(1_000_000);
        Self { inner }
    }
}

impl ObservedOpsBehaviors {
    pub fn merge(&mut self, new_data: &OpsBehaviorData) -> bool {
        !self.inner.insert(new_data)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpsBehaviorObserver {
    name: Cow<'static, str>,
    max_syn_depth: usize,
    #[serde(skip)]
    observed_behavior: Option<HashSet<OpsBehaviorData>>,
}

impl OpsBehaviorObserver {
    pub fn new<N>(name: N, max_syn_depth: usize) -> Self
    where
        N: Into<Cow<'static, str>>,
    {
        Self {
            name: name.into(),
            max_syn_depth,
            observed_behavior: None,
        }
    }

    pub const fn observed_behavior(&self) -> Option<&HashSet<OpsBehaviorData>> {
        self.observed_behavior.as_ref()
    }
}

impl Named for OpsBehaviorObserver {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<State> Observer<LspInput, State> for OpsBehaviorObserver {
    fn pre_exec(&mut self, _state: &mut State, _input: &LspInput) -> Result<(), libafl::Error> {
        self.observed_behavior = None;
        Ok(())
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        input: &LspInput,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<(), libafl::Error> {
        let data = analyze_behavior_data(input, self.max_syn_depth)
            .afl_context("Analyzing behavior data")?;
        self.observed_behavior = Some(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lsp_fuzz_grammars::Language;

    #[test]
    fn bloom_filter() {
        let mut bloom = fastbloom::BloomFilter::with_false_pos(0.01).expected_items(10);
        assert!(!bloom.insert(&233));
    }

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
