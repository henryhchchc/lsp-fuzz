use std::borrow::Cow;

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

        const MAX_PATH_SEGMENTS: usize = 3;
        let action = state.rand_mut().below_incl(MAX_ACTIONS);
        match action {
            NEW_FILE => {
                let mut new_file = BytesInput::default();
                self.inner_mutator.mutate(state, &mut new_file)?;
                let new_file_path = {
                    let path_segments = state.rand_mut().between(1, MAX_PATH_SEGMENTS);
                    let segments = (0..path_segments)
                        .map(|_| {
                            let mut segment = PathSegmentInput::default();
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
                // TODO: Implement mutation logic for existing files
                Ok(MutationResult::Skipped)
            }
            REMOVE_FILE => {
                // TODO: Implement logic to remove a file
                Ok(MutationResult::Skipped)
            }
            RELOCATE_FILE => {
                // TODO: Implement logic to relocate a file
                Ok(MutationResult::Skipped)
            }
            MAX_ACTIONS.. => unreachable!("Garenteed by RNG"),
        }
    }
}
