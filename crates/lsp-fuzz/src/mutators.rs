use std::{borrow::Cow, collections::BTreeSet, marker::PhantomData};

use libafl::{
    mutators::{ComposedByMutations, MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, Named};

#[derive(Debug)]
pub struct ShortCurcuitMutator<I, MT, S> {
    mutators: MT,
    _phantom: PhantomData<(I, S)>,
}

impl<I, MT, S> ShortCurcuitMutator<I, MT, S> {
    pub fn new(mutators: MT) -> Self {
        Self {
            mutators,
            _phantom: PhantomData,
        }
    }
}

impl<I, MT, S> Named for ShortCurcuitMutator<I, MT, S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ShortCurcuitMutator");
        &NAME
    }
}

impl<I, MT, S> ComposedByMutations for ShortCurcuitMutator<I, MT, S> {
    type Mutations = MT;

    fn mutations(&self) -> &Self::Mutations {
        &self.mutators
    }

    fn mutations_mut(&mut self) -> &mut Self::Mutations {
        &mut self.mutators
    }
}

impl<I, MT, S> Mutator<I, S> for ShortCurcuitMutator<I, MT, S>
where
    I: Clone,
    MT: MutatorsTuple<I, S>,
    S: HasRand,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let mut rand = state.rand_mut();
        let mut mutator_idx: BTreeSet<_> = (0..self.mutators.len()).collect();
        while let Some(&idx) = rand.choose(mutator_idx.iter()) {
            mutator_idx.remove(&idx);
            match self.mutators.get_and_mutate(idx.into(), state, input)? {
                MutationResult::Mutated => return Ok(MutationResult::Mutated),
                MutationResult::Skipped => rand = state.rand_mut(),
            }
        }
        Ok(MutationResult::Skipped)
    }
}
