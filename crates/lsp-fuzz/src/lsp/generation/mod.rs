use std::{
    marker::{PhantomData, Sized},
    ops::Deref,
    result::Result,
};

use libafl::state::HasRand;
use libafl_bolts::rands::Rand;

use crate::lsp_input::LspInput;

use super::HasPredefinedGenerators;
pub mod composition;
pub mod consts;
pub mod doc;
pub mod doc_range;
pub mod position;
pub mod string;

pub trait LspParamsGenerator<State> {
    type Output;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError>;
}

impl<State, G, Ptr> LspParamsGenerator<State> for Ptr
where
    Ptr: Deref<Target = G>,
    G: LspParamsGenerator<State> + ?Sized,
{
    type Output = G::Output;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        self.deref().generate(state, input)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("Nothing was generated")]
    NothingGenerated,
    #[error(transparent)]
    Error(#[from] libafl::Error),
}

#[derive(Debug)]
pub struct DefaultGenerator<T> {
    _phantom: PhantomData<T>,
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

#[derive(Debug)]
pub struct ZeroToOne32(pub f32);

#[derive(Debug, Clone)]
pub struct ZeroToOne32Gen;

impl<State> LspParamsGenerator<State> for ZeroToOne32Gen
where
    State: HasRand,
{
    type Output = ZeroToOne32;

    fn generate(
        &self,
        state: &mut State,
        _input: &LspInput,
    ) -> Result<ZeroToOne32, GenerationError> {
        Ok(ZeroToOne32(state.rand_mut().next_float() as f32))
    }
}

impl<State> HasPredefinedGenerators<State> for ZeroToOne32
where
    State: HasRand,
{
    type Generator = ZeroToOne32Gen;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: 'static,
    {
        [ZeroToOne32Gen]
    }
}

#[derive(Debug)]
pub struct TabSize(pub u32);

#[derive(Debug, Clone)]
pub struct TabSizeGen;

impl<State> LspParamsGenerator<State> for TabSizeGen
where
    State: HasRand,
{
    type Output = TabSize;

    fn generate(&self, state: &mut State, _input: &LspInput) -> Result<TabSize, GenerationError> {
        let inner = match state.rand_mut().next() % 6 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 4,
            4 => 8,
            5 => state.rand_mut().next() as u32,
            _ => unreachable!("Modulo of 6 should not be greater than 5"),
        };
        Ok(TabSize(inner))
    }
}

impl<State> HasPredefinedGenerators<State> for TabSize
where
    State: HasRand,
{
    type Generator = TabSizeGen;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: 'static,
    {
        [TabSizeGen]
    }
}
