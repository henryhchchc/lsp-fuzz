use libafl::state::HasRand;
use libafl_bolts::rands::Rand;
use serde::{Deserialize, Serialize};

use super::{GenerationError, LspParamsGenerator};
use crate::{
    lsp::{GeneratorsConfig, HasPredefinedGenerators},
    lsp_input::LspInput,
};

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

    fn generators(_config: &GeneratorsConfig) -> impl IntoIterator<Item = Self::Generator> {
        [ZeroToOne32Gen]
    }
}

#[derive(Debug)]
pub struct TabSize(pub u32);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TabSizeGen {
    pub candidates: Vec<u32>,
    pub rand_prob: f64,
}

impl<State> LspParamsGenerator<State> for TabSizeGen
where
    State: HasRand,
{
    type Output = TabSize;

    fn generate(&self, state: &mut State, _input: &LspInput) -> Result<TabSize, GenerationError> {
        let rand = state.rand_mut();
        let value = rand
            .choose(&self.candidates)
            .copied()
            .unwrap_or_else(|| rand.next() as u32);
        Ok(TabSize(value))
    }
}

impl<State> HasPredefinedGenerators<State> for TabSize
where
    State: HasRand,
{
    type Generator = TabSizeGen;

    fn generators(config: &GeneratorsConfig) -> impl IntoIterator<Item = Self::Generator> {
        [config.tab_size.clone()]
    }
}
