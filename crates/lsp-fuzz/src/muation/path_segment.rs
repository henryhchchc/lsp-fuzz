use std::{borrow::Cow, num::NonZero};

use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, Named};

use crate::inputs::path_segment::PathSegmentInput;

#[derive(Debug)]
pub struct PathSegmentMutator;

impl PathSegmentMutator {}

impl Named for PathSegmentMutator {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("Utf8Mutator")
    }
}

impl<S> Mutator<PathSegmentInput, S> for PathSegmentMutator
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut PathSegmentInput,
    ) -> Result<MutationResult, libafl::Error> {
        let new_char = {
            let mut the_char = Some('/');
            while matches!(the_char, None | Some('/')) {
                let code_point = state.rand_mut().between(1usize, char::MAX as usize) as u32;
                the_char = std::char::from_u32(code_point);
            }
            the_char.unwrap()
        };

        const APPEND_CHAR: usize = 0;
        const DELETE_CHAR: usize = 1;
        const INSERT_CHAR: usize = 2;
        const PREPEND_CHAR: usize = 3;
        const MAX_ACTIONS: usize = 4;
        let action = state.rand_mut().below(NonZero::new(MAX_ACTIONS).unwrap());

        match action {
            APPEND_CHAR => {
                input.inner.push(new_char);
                Ok(MutationResult::Mutated)
            }
            DELETE_CHAR => {
                if !input.inner.is_empty() {
                    let idx = state
                        .rand_mut()
                        .below(NonZero::new(input.inner.len()).unwrap());
                    input.inner.remove(idx);
                    Ok(MutationResult::Mutated)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            INSERT_CHAR => {
                let idx = state.rand_mut().below_incl(input.inner.len());
                input.inner.insert(idx, new_char);
                Ok(MutationResult::Mutated)
            }
            PREPEND_CHAR => {
                input.inner.insert(0, new_char);
                Ok(MutationResult::Mutated)
            }
            MAX_ACTIONS.. => unreachable!("Garenteed by RNG"),
        }
    }
}
