use std::{borrow::Cow, collections::BTreeSet, marker::PhantomData, num::NonZero, sync::OnceLock};

use derive_new::new as New;
use libafl::{
    mutators::{ComposedByMutations, MutationResult, Mutator, MutatorsTuple},
    state::HasRand,
};
use libafl_bolts::{Named, rands::Rand};

#[derive(Debug, New)]
pub struct ShortCurcuitMutator<I, MT, State> {
    mutators: MT,
    _phantom: PhantomData<(I, State)>,
}

impl<I, MT, State> Named for ShortCurcuitMutator<I, MT, State> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ShortCurcuitMutator");
        &NAME
    }
}

impl<I, MT, State> ComposedByMutations for ShortCurcuitMutator<I, MT, State> {
    type Mutations = MT;

    fn mutations(&self) -> &Self::Mutations {
        &self.mutators
    }

    fn mutations_mut(&mut self) -> &mut Self::Mutations {
        &mut self.mutators
    }
}

impl<I, MT, State> Mutator<I, State> for ShortCurcuitMutator<I, MT, State>
where
    I: Clone,
    MT: MutatorsTuple<I, State>,
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
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
pub struct SliceSwapMutator<T, State> {
    _item: PhantomData<T>,
    _state: PhantomData<State>,
}

impl<State, T> Named for SliceSwapMutator<T, State> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("SliceSwapMutator");
        &NAME
    }
}

impl<I, T, State> Mutator<I, State> for SliceSwapMutator<T, State>
where
    I: AsMut<[T]>,
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
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
pub struct PropMutator<PM, const PROP_ID: usize> {
    mutator: PM,
}

impl<M, const PROP_ID: usize> Named for PropMutator<M, PROP_ID> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("FieldMutator");
        &NAME
    }
}

impl<I, T, M, State, const PROP_ID: usize> Mutator<I, State> for PropMutator<M, PROP_ID>
where
    M: Mutator<T, State>,
    State: HasRand,
    I: HasMutProp<PROP_ID, PropType = T>,
{
    #[inline]
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        let field_mut = I::get_field(input);
        self.mutator.mutate(state, field_mut)
    }
}

impl<PM, const PROP_ID: usize> ComposedByMutations for PropMutator<PM, PROP_ID> {
    type Mutations = PM;

    fn mutations(&self) -> &Self::Mutations {
        &self.mutator
    }

    fn mutations_mut(&mut self) -> &mut Self::Mutations {
        &mut self.mutator
    }
}

#[derive(Debug)]
pub struct OptionMutator<M> {
    mutator: M,
    name: OnceLock<Cow<'static, str>>,
}

impl<M> OptionMutator<M> {
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            name: OnceLock::default(),
        }
    }
}

impl<M> Named for OptionMutator<M>
where
    M: Named,
{
    fn name(&self) -> &Cow<'static, str> {
        self.name.get_or_init(|| {
            let name = format!("OptionMutator<{}>", self.mutator.name());
            Cow::Owned(name)
        })
    }
}

impl<I, M, State> Mutator<Option<I>, State> for OptionMutator<M>
where
    M: Mutator<I, State>,
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut Option<I>,
    ) -> Result<MutationResult, libafl::Error> {
        input
            .as_mut()
            .map(|it| self.mutator.mutate(state, it))
            .unwrap_or(Ok(MutationResult::Skipped))
    }
}
