use libafl::state::HasRand;
use libafl_bolts::rands::Rand;

use super::{GenerationError, LspParamsGenerator};
use crate::{lsp::HasPredefinedGenerators, lsp_input::LspInput};

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

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
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

    fn generators() -> impl IntoIterator<Item = Self::Generator> {
        [TabSizeGen]
    }
}
