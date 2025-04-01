use ahash::AHasher;
use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    hash::Hasher,
};

use libafl::{
    HasMetadata, SerdeAny,
    feedbacks::{Feedback, StateInitializer},
};
use libafl_bolts::Named;

use crate::{lsp_input::LspInput, utils::AflContext};

use super::Language;

#[derive(Debug, New)]
pub struct TokenNoveltyFeedback {
    max_depth: usize,
}

impl Named for TokenNoveltyFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("TokenNoveltyFeedback");
        &NAME
    }
}

impl<S> StateInitializer<S> for TokenNoveltyFeedback {}

impl<EM, OBS, S> Feedback<EM, LspInput, OBS, S> for TokenNoveltyFeedback
where
    S: HasMetadata,
{
    fn is_interesting(
        &mut self,
        state: &mut S,
        _manager: &mut EM,
        input: &LspInput,
        _observers: &OBS,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        let text_document = input
            .workspace
            .iter_files()
            .filter_map(|it| it.1.as_source_file())
            .next()
            .afl_context("No text document found")?;
        let parse_tree = text_document
            .parse_tree()
            .ok_or(libafl::Error::illegal_state(
                "Assumption violated: parse tree should be available upon token novelty evaluation",
            ))?;
        let seen_hashes = state.metadata_or_insert_with(SeenTokenHashes::default);
        if let Some(token_hashes) = hash_paths(parse_tree, self.max_depth) {
            let is_interesting = seen_hashes.update(text_document.language, token_hashes);
            Ok(is_interesting)
        } else {
            Ok(false)
        }
    }
}

fn hash_paths(parse_tree: &tree_sitter::Tree, max_depth: usize) -> Option<HashSet<u64>> {
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

#[derive(Debug, Serialize, Deserialize, Deref, DerefMut, Default, SerdeAny)]
pub struct SeenTokenHashes {
    inner: HashMap<Language, HashSet<u64>>,
}

impl SeenTokenHashes {
    pub fn update(&mut self, language: Language, token_hashes: HashSet<u64>) -> bool {
        let lang_seen_hashes = self.inner.entry(language).or_default();
        if lang_seen_hashes.is_superset(&token_hashes) {
            false
        } else {
            lang_seen_hashes.extend(token_hashes);
            true
        }
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
