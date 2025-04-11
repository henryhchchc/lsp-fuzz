use std::{iter::once, marker::PhantomData, rc::Rc};

use lsp_types::OneOf;

use super::{GenerationError, LspParamsGenerator};
use crate::{
    lsp::HasPredefinedGenerators,
    lsp_input::{LspInput, messages::OptionGenerator},
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

#[derive(Debug)]
pub struct MappingGenerator<State, G, T, U> {
    generator: G,
    mapper: fn(T) -> U,
    _phantom: PhantomData<State>,
}

impl<State, G, T, U> MappingGenerator<State, G, T, U> {
    pub const fn new(generator: G, mapper: fn(T) -> U) -> Self {
        Self {
            generator,
            mapper,
            _phantom: PhantomData,
        }
    }
}

impl<State, G, T, U> Clone for MappingGenerator<State, G, T, U>
where
    G: Clone,
{
    fn clone(&self) -> Self {
        let generator = self.generator.clone();
        Self::new(generator, self.mapper)
    }
}

impl<State, G, T, U> LspParamsGenerator<State> for MappingGenerator<State, G, T, U>
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
    State: 'static,
    A: HasPredefinedGenerators<State> + 'static,
    B: HasPredefinedGenerators<State> + 'static,
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
    State: 'static,
    T: HasPredefinedGenerators<State> + 'static,
    T::Generator: Clone,
{
    type Generator = OptionGenerator<State, T>;

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        T::generators()
            .into_iter()
            .flat_map(|g| [OptionGenerator::new(Some(g))])
            .chain(once(OptionGenerator::new(None)))
    }
}
