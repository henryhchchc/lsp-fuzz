use std::marker::PhantomData;

use libafl::{
    HasMetadata,
    state::{HasCurrentTestcase, HasRand},
};
use libafl_bolts::rands::Rand;
use lsp_types::OneOf;

use super::{GenerationError, LspParamsGenerator};
use crate::{
    lsp::HasPredefinedGenerators,
    lsp_input::{
        LspInput,
        server_response::metadata::{ContainsFragment, ParamFragments},
    },
};

#[derive(Debug)]
pub struct DefaultGenerator<T> {
    _phantom: PhantomData<fn() -> T>,
}

impl<T> DefaultGenerator<T> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for DefaultGenerator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for DefaultGenerator<T> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<State, T> LspParamsGenerator<State> for DefaultGenerator<T>
where
    T: Default,
{
    type Output = T;

    fn generate(
        &self,
        _state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        Ok(T::default())
    }
}

#[derive(Debug, Clone)]
pub enum OneOfGenerator<L, R> {
    Left(L),
    Right(R),
}

impl<State, L, R> LspParamsGenerator<State> for OneOfGenerator<L, R>
where
    L: LspParamsGenerator<State, Output = L>,
    R: LspParamsGenerator<State, Output = R>,
{
    type Output = OneOf<L, R>;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        match self {
            OneOfGenerator::Left(lgen) => Ok(OneOf::Left(lgen.generate(state, input)?)),
            OneOfGenerator::Right(rgen) => Ok(OneOf::Right(rgen.generate(state, input)?)),
        }
    }
}

impl<State, A, B> HasPredefinedGenerators<State> for OneOf<A, B>
where
    A: HasPredefinedGenerators<State>,
    B: HasPredefinedGenerators<State>,
    OneOfGenerator<A::Generator, B::Generator>: LspParamsGenerator<State, Output = OneOf<A, B>>,
{
    type Generator = OneOfGenerator<A::Generator, B::Generator>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        let left_gen = A::generators(config).into_iter().map(OneOfGenerator::Left);
        let right_gen = B::generators(config).into_iter().map(OneOfGenerator::Right);
        left_gen.chain(right_gen)
    }
}

impl<State, T> HasPredefinedGenerators<State> for Option<T>
where
    T: HasPredefinedGenerators<State>,
    OptionGenerator<T::Generator>: LspParamsGenerator<State, Output = Option<T>>,
{
    type Generator = OptionGenerator<T::Generator>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        T::generators(config)
            .into_iter()
            .map(|inner| OptionGenerator::new(inner, 0.2))
    }
}

#[derive(Debug)]
pub struct OptionGenerator<G> {
    inner: G,
    none_prob: f64,
}

impl<G> OptionGenerator<G> {
    pub const fn new(inner: G, none_prob: f64) -> Self {
        Self { inner, none_prob }
    }
}

impl<G> Clone for OptionGenerator<G>
where
    G: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.inner.clone(), self.none_prob)
    }
}

impl<State, G> LspParamsGenerator<State> for OptionGenerator<G>
where
    State: HasRand,
    G: LspParamsGenerator<State>,
{
    type Output = Option<G::Output>;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        if state.rand_mut().coinflip(self.none_prob) {
            Ok(None)
        } else {
            self.inner.generate(state, input).map(Some)
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ParamFragmentGenerator<T> {
    enabled: bool,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> ParamFragmentGenerator<T> {
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            _phantom: PhantomData,
        }
    }
}

impl<State, T> LspParamsGenerator<State> for ParamFragmentGenerator<T>
where
    State: HasRand + HasCurrentTestcase<LspInput>,
    ParamFragments: ContainsFragment<T>,
    T: Clone,
{
    type Output = T;

    fn generate(
        &self,
        state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        if !self.enabled {
            return Err(GenerationError::NothingGenerated);
        }
        let testcase = state
            .current_testcase()
            .map_err(|_| GenerationError::NothingGenerated)?;

        let fragment_store = testcase
            .metadata::<ParamFragments>()
            .map_err(|_| GenerationError::NothingGenerated)?;

        let frag_len = fragment_store.fragments().len();
        drop(testcase);
        let selected_idx = state
            .rand_mut()
            .choose(0..frag_len)
            .ok_or(GenerationError::NothingGenerated)?;

        // We have to reborrow here. This is a pain in the ass.

        let testcase = state
            .current_testcase()
            .map_err(|_| GenerationError::NothingGenerated)?;

        let fragment_store = testcase
            .metadata::<ParamFragments>()
            .map_err(|_| GenerationError::NothingGenerated)?;
        let param_fragments = fragment_store.fragments();
        let generated: &T = param_fragments
            .iter()
            .nth(selected_idx)
            .expect("The index is witin the range");
        Ok(generated.clone())
    }
}
