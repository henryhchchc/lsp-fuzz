use libafl::state::HasRand;
use libafl_bolts::rands::Rand;

use super::{GenerationError, HasGenerators, LspParamsGenerator};
use crate::lsp_input::LspInput;

#[derive(Debug, Clone)]
pub struct VecGenerator<G> {
    element_generators: Vec<G>,
    max_items: usize,
}

impl<G> VecGenerator<G> {
    #[must_use]
    pub fn new(element_generators: impl IntoIterator<Item = G>, max_items: usize) -> Self {
        let element_generators = element_generators.into_iter().collect();
        Self {
            element_generators,
            max_items,
        }
    }
}

impl<State, G> LspParamsGenerator<State> for VecGenerator<G>
where
    G: LspParamsGenerator<State>,
    State: HasRand,
{
    type Output = Vec<G::Output>;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let len = state.rand_mut().below_or_zero(self.max_items);
        let mut items = Vec::with_capacity(len);
        let mut anything_generated = false;
        for _ in 0..len {
            if let Some(generator) = state.rand_mut().choose(&self.element_generators) {
                match generator.generate(state, input) {
                    Ok(item) => {
                        items.push(item);
                        anything_generated = true;
                    }
                    Err(GenerationError::NothingGenerated) => {}
                    Err(err) => return Err(err),
                }
            }
        }

        anything_generated
            .then_some(items)
            .ok_or(GenerationError::NothingGenerated)
    }
}

impl<State, T> HasGenerators<State> for Vec<T>
where
    State: HasRand,
    T: HasGenerators<State>,
{
    type Generator = VecGenerator<T::Generator>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        [VecGenerator::new(T::generators(config), 5)]
    }
}
