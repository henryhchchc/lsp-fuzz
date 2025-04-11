use std::{marker::PhantomData, rc::Rc};

use libafl::state::HasRand;
use libafl_bolts::rands::Rand;
use lsp_types::OneOf;

use super::{GenerationError, LspParamsGenerator};
use crate::{lsp::HasPredefinedGenerators, lsp_input::LspInput};

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

#[derive(Debug)]
pub struct MappingGenerator<G, T, U> {
    generator: G,
    mapper: fn(T) -> U,
}

impl<G, T, U> MappingGenerator<G, T, U> {
    pub const fn new(generator: G, mapper: fn(T) -> U) -> Self {
        Self { generator, mapper }
    }
}

impl<G, T, U> Clone for MappingGenerator<G, T, U>
where
    G: Clone,
{
    fn clone(&self) -> Self {
        let generator = self.generator.clone();
        Self::new(generator, self.mapper)
    }
}

impl<State, G, T, U> LspParamsGenerator<State> for MappingGenerator<G, T, U>
where
    G: LspParamsGenerator<State, Output = T>,
{
    type Output = U;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        self.generator.generate(state, input).map(self.mapper)
    }
}

impl<State, A, B> HasPredefinedGenerators<State> for OneOf<A, B>
where
    A: HasPredefinedGenerators<State> + 'static,
    B: HasPredefinedGenerators<State> + 'static,
    A::Generator: 'static,
    B::Generator: 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<State, Output = Self>>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        let left_gen = A::generators()
            .into_iter()
            .map(|g| Rc::new(MappingGenerator::new(g, OneOf::Left)) as _);
        let right_gen = B::generators()
            .into_iter()
            .map(|g| Rc::new(MappingGenerator::new(g, OneOf::Right)) as _);
        left_gen.chain(right_gen)
    }
}

impl<State, T> HasPredefinedGenerators<State> for Option<T>
where
    T: HasPredefinedGenerators<State>,
    OptionGenerator<T::Generator>: LspParamsGenerator<State, Output = Option<T>>,
{
    type Generator = OptionGenerator<T::Generator>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        T::generators()
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
