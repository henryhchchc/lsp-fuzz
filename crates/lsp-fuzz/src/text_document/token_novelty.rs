use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use libafl::{
    feedbacks::{Feedback, StateInitializer},
    HasMetadata,
};
use libafl_bolts::{impl_serdeany, Named};

use crate::{lsp_input::LspInput, utils::AflContext};

use super::Language;

#[derive(Debug, New)]
pub struct TokenNoveltyFeedback {}

impl Named for TokenNoveltyFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("TokenNoveltyFeedback");
        &NAME
    }
}

impl<S> StateInitializer<S> for TokenNoveltyFeedback {}

impl<EM, OT, S> Feedback<EM, LspInput, OT, S> for TokenNoveltyFeedback
where
    S: HasMetadata,
{
    fn is_interesting(
        &mut self,
        state: &mut S,
        _manager: &mut EM,
        input: &LspInput,
        _observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        let text_document = input
            .source_directory
            .iter_files()
            .map(|it| it.1)
            .next()
            .afl_context("No text document found")?;
        let Some(parse_tree) = text_document.parse_tree() else {
            // TODO: Maybe return an error here?
            return Ok(false);
        };
        let seen_hashes = state.metadata_or_insert_with(SeenTokenHashes::default);
        let token_hashes = hash_paths(parse_tree);
        let is_interesting = seen_hashes.update(text_document.language, token_hashes);
        Ok(is_interesting)
    }
}

fn hash_paths(parse_tree: &tree_sitter::Tree) -> HashSet<u64> {
    let mut hashes = HashSet::new();
    todo!("To be done tomorrow ™");
    hashes
}

#[derive(Debug, Serialize, Deserialize, Deref, DerefMut, Default)]
pub struct SeenTokenHashes {
    inner: HashMap<Language, HashSet<u64>>,
}

impl_serdeany!(SeenTokenHashes);

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
