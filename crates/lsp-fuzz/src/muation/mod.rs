use std::{borrow::Cow, num::NonZero};

use libafl::{
    inputs::{BytesInput, UsesInput},
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand, State},
    HasMetadata,
};
use libafl_bolts::{rands::Rand, Named};
use path_segment::PathSegmentMutator;

use crate::inputs::{path_segment::PathSegmentInput, LspInput, PathInput};

pub mod path_segment;

#[derive(Debug)]
pub struct LspInputMutator<M> {
    inner_mutator: M,
}

impl<M> LspInputMutator<M> {
    pub fn new(inner_mutator: M) -> Self {
        Self { inner_mutator }
    }
}

impl<M> Named for LspInputMutator<M> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &Cow::Borrowed("LspInputMutator")
    }
}

impl<M, S> Mutator<LspInput, S> for LspInputMutator<M>
where
    M: Mutator<BytesInput, S>,
    S: State + UsesInput<Input = LspInput> + HasMetadata + HasCorpus + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        const NEW_FILE: usize = 0;
        const MUTATE_FILE: usize = 1;
        const REMOVE_FILE: usize = 2;
        const RELOCATE_FILE: usize = 3;
        const MAX_ACTIONS: usize = 4;

        const MAX_PATH_SEGMENTS: usize = 1;
        let action = state.rand_mut().below(NonZero::new(MAX_ACTIONS).unwrap());
        match action {
            NEW_FILE => {
                let mut new_file = BytesInput::default();
                self.inner_mutator.mutate(state, &mut new_file)?;
                let new_file_path = {
                    let path_segments = state.rand_mut().between(1, MAX_PATH_SEGMENTS);
                    let segments = (0..path_segments)
                        .map(|_| {
                            let mut segment = PathSegmentInput::new("main.c".to_owned());
                            PathSegmentMutator::new()
                                .mutate(state, &mut segment)
                                .unwrap();
                            segment
                        })
                        .collect();
                    PathInput { segments }
                };
                input.source_directory.insert(new_file_path, new_file);
                Ok(MutationResult::Mutated)
            }
            MUTATE_FILE => {
                if let Some((_path, file)) = input.source_directory.iter_mut().next() {
                    self.inner_mutator.mutate(state, file)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            REMOVE_FILE => {
                if !input.source_directory.is_empty() {
                    let keys: Vec<_> = input.source_directory.keys().cloned().collect();
                    let key_to_remove = state.rand_mut().choose(&keys).unwrap();
                    input.source_directory.remove(key_to_remove);
                    Ok(MutationResult::Mutated)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            RELOCATE_FILE => {
                if !input.source_directory.is_empty() {
                    let keys: Vec<_> = input.source_directory.keys().cloned().collect();
                    let key_to_relocate = state.rand_mut().choose(&keys).unwrap();
                    let new_path = {
                        let path_segments = state.rand_mut().between(1, MAX_PATH_SEGMENTS);
                        let segments = (0..path_segments)
                            .map(|_| {
                                let mut segment = PathSegmentInput::new("main.c".to_owned());
                                PathSegmentMutator::new()
                                    .mutate(state, &mut segment)
                                    .unwrap();
                                segment
                            })
                            .collect();
                        PathInput { segments }
                    };
                    if input.source_directory.contains_key(&new_path) {
                        Ok(MutationResult::Skipped)
                    } else {
                        let file = input.source_directory.remove(key_to_relocate).unwrap();
                        input.source_directory.insert(new_path, file);
                        Ok(MutationResult::Mutated)
                    }
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            MAX_ACTIONS.. => unreachable!("Garenteed by RNG"),
        }
    }
}
