use std::{borrow::Cow, collections::BTreeSet, marker::PhantomData, num::NonZero};

use derive_new::new as New;
use libafl::{
    mutators::{ComposedByMutations, MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, Named};

#[derive(Debug, New)]
pub struct ShortCurcuitMutator<I, MT, S> {
    mutators: MT,
    _phantom: PhantomData<(I, S)>,
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

#[derive(Debug, New)]
pub struct SliceSwapMutator<T, S> {
    _item: PhantomData<T>,
    _state: PhantomData<S>,
}

impl<S, T> Named for SliceSwapMutator<T, S> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("SliceSwapMutator");
        &NAME
    }
}

impl<I, T, S> Mutator<I, S> for SliceSwapMutator<T, S>
where
    I: AsMut<[T]>,
    S: HasRand,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let input = input.as_mut();
        let len = input.len();
        if len < 2 {
            return Ok(MutationResult::Skipped);
        }
        // Safety: We just checked that len >= 2
        let len = unsafe { NonZero::new_unchecked(len) };
        let rand = state.rand_mut();
        let idx1 = rand.below(len);
        let idx2 = rand.below(len);
        input.swap(idx1, idx2);
        Ok(MutationResult::Mutated)
    }
}

pub trait HasMutProp<const OFFSET: usize> {
    type PropType;

    fn get_field(&mut self) -> &mut Self::PropType;
}

#[derive(Debug, New)]
pub struct PropMutator<PM, const OFFSET: usize> {
    mutator: PM,
}

impl<M, const OFFSET: usize> Named for PropMutator<M, OFFSET> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("FieldMutator");
        &NAME
    }
}

impl<I, T, M, S, const OFFSET: usize> Mutator<I, S> for PropMutator<M, OFFSET>
where
    M: Mutator<T, S>,
    S: HasRand,
    I: HasMutProp<OFFSET, PropType = T>,
{
    #[inline]
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let field_mut = I::get_field(input);
        self.mutator.mutate(state, field_mut)
    }
}
