use libafl::state::HasRand;
use libafl_bolts::rands::Rand;

pub mod core;
pub mod node_filters;
pub mod node_generators;

pub use core::{NodeContentMutator, NodeGenerator, NodeSelector};

pub const MAX_DOCUMENT_SIZE: usize = 100_000;

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
        *content = string.into_bytes();
    }
}
