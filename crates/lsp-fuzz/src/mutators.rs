use std::{borrow::Cow, marker::PhantomData, num::NonZero, sync::OnceLock};

use derive_new::new as New;
use libafl::{
    corpus::CorpusId,
    mutators::{ComposedByMutations, MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{Named, rands::Rand};

#[derive(Debug)]
pub struct FallbackMutator<Frist, Second> {
    first: Frist,
    second: Second,
}

impl<Frist, Second> FallbackMutator<Frist, Second> {
    pub const fn new(first: Frist, second: Second) -> Self {
        Self { first, second }
    }
}

impl<First, Second> Named for FallbackMutator<First, Second>
where
    First: Named,
    Second: Named,
{
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("FallbackMutator");
        &NAME
    }
}

impl<I, First, Second, State> Mutator<I, State> for FallbackMutator<First, Second>
where
    First: Mutator<I, State>,
    Second: Mutator<I, State>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        let first_reult = self.first.mutate(state, input)?;
        if first_reult == MutationResult::Skipped {
            self.second.mutate(state, input)
        } else {
            Ok(first_reult)
        }
    }

    fn post_exec(
        &mut self,
        state: &mut State,
        new_corpus_id: Option<CorpusId>,
    ) -> Result<(), libafl::Error> {
        self.first.post_exec(state, new_corpus_id)?;
        self.second.post_exec(state, new_corpus_id)?;
        Ok(())
    }
}

pub trait WithProbability {
    fn with_probability(self, probability: f64) -> ProbabilityMutator<Self>
    where
        Self: Sized;
}

impl<M> WithProbability for M {
    fn with_probability(self, probability: f64) -> ProbabilityMutator<Self>
    where
        Self: Sized,
    {
        ProbabilityMutator::new(self, probability)
    }
}

#[derive(Debug, New)]
pub struct ProbabilityMutator<Inner> {
    inner: Inner,
    probability: f64,
}

impl<Inner> Named for ProbabilityMutator<Inner>
where
    Inner: Named,
{
    fn name(&self) -> &Cow<'static, str> {
        self.inner.name()
    }
}

impl<Inner, State, I> Mutator<I, State> for ProbabilityMutator<Inner>
where
    State: HasRand,
    Inner: Mutator<I, State>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        // make it faster by skipping the rand generation if prob is zero
        if self.probability == 0.0 || !state.rand_mut().coinflip(self.probability) {
            Ok(MutationResult::Skipped)
        } else {
            self.inner.mutate(state, input)
        }
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
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

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
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

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
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

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}
