use std::iter::repeat;

use derive_more::derive::{Deref, DerefMut};
use derive_new::new as New;
use libafl::HasMetadata;
use libafl_bolts::rands::Rand;
use lsp_fuzz_grammars::WELL_KNOWN_HIGHLIGHT_CAPTURE_NAMES;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    text_document::{
        TextDocument,
        grammar::tree_sitter::{CapturesIterator, TSNodeExt},
    },
    utils::{RandExt, ToLspPosition},
};

pub trait PositionSelector<State> {
    fn select_position(&self, state: &mut State, doc: &TextDocument)
    -> Option<lsp_types::Position>;
}

#[derive(Debug, New)]
pub struct RandomPosition {
    rand_max: usize,
}

impl<State> PositionSelector<State> for RandomPosition
where
    State: libafl::state::HasRand,
{
    fn select_position(
        &self,
        state: &mut State,
        _doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let rand = state.rand_mut();
        let line = u32::try_from(rand.between(0, self.rand_max)).ok()?;
        let character = u32::try_from(rand.between(0, self.rand_max)).ok()?;
        Some(lsp_types::Position { line, character })
    }
}

#[derive(Debug, New)]
pub struct ValidPosition;

impl<State> PositionSelector<State> for ValidPosition
where
    State: libafl::state::HasRand,
{
    fn select_position(
        &self,
        state: &mut State,
        doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let positions = doc
            .lines()
            .enumerate()
            .flat_map(|(idx, line)| (0..line.len()).map(move |char| (idx, char)));
        let (line, char) = state.rand_mut().choose(positions)?;
        Some(lsp_types::Position {
            line: u32::try_from(line).ok()?,
            character: u32::try_from(char).ok()?,
        })
    }
}

#[derive(Debug, Clone, Copy, New)]
pub struct NodeTypeBalancingSelection;

impl<State> PositionSelector<State> for NodeTypeBalancingSelection
where
    State: libafl::state::HasRand,
{
    fn select_position(
        &self,
        state: &mut State,
        doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let (_signature, nodes) = state.rand_mut().choose(&doc.metadata().node_signatures)?;
        let node = state.rand_mut().choose(nodes)?;
        Some(node.to_lsp_position())
    }
}

#[derive(Debug, Clone, Copy, New)]
pub struct HighlightSteer;

#[derive(Debug, Serialize, Deref, DerefMut, libafl_bolts::SerdeAny)]
pub struct HighlightGroupUsageMetadata {
    #[deref]
    #[deref_mut]
    inner: ahash::HashMap<String, usize>,
}

impl<'de> Deserialize<'de> for HighlightGroupUsageMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct HighlightGroupUsageMetadataRepr {
            inner: ahash::HashMap<String, usize>,
        }

        HighlightGroupUsageMetadataRepr::deserialize(deserializer)
            .map(|repr| Self { inner: repr.inner })
    }
}

impl HighlightGroupUsageMetadata {
    #[must_use]
    pub fn new<Names, Name>(highlight_group_names: Names) -> Self
    where
        Names: IntoIterator<Item = Name>,
        Name: Into<String>,
    {
        let inner = highlight_group_names
            .into_iter()
            .map(Into::into)
            .zip(repeat(0))
            .collect();
        Self { inner }
    }
}

impl<State> PositionSelector<State> for HighlightSteer
where
    State: libafl::state::HasRand + HasMetadata,
{
    fn select_position(
        &self,
        state: &mut State,
        doc: &TextDocument,
    ) -> Option<lsp_types::Position> {
        let usage_stats = state.metadata_or_insert_with(|| {
            HighlightGroupUsageMetadata::new(WELL_KNOWN_HIGHLIGHT_CAPTURE_NAMES)
        });
        let max_usage = usage_stats.values().copied().max().unwrap_or_default();
        let weights: Vec<_> = usage_stats
            .iter()
            .map(|(name, &usage)| (name.clone(), max_usage - usage))
            .collect();
        let chosen = state.rand_mut().weighted_choose(weights)?;
        let captures = CapturesIterator::new(doc, &chosen)?;
        let node = state.rand_mut().choose(captures)?;
        let pos = node.lsp_start_position();
        let usage_stats = state
            .metadata_mut::<HighlightGroupUsageMetadata>()
            .expect("We ensured it is inserted");
        let usage = usage_stats
            .get_mut(&chosen)
            .expect("The entry is in the map");
        *usage += 1;
        Some(pos)
    }
}
