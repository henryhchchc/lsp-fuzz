use std::marker::PhantomData;

use super::{GenerationError, LspParamsGenerator};
use crate::{
    lsp::{Compose, HasPredefinedGenerators},
    lsp_input::LspInput,
};

impl<State, T, T1, T2> HasPredefinedGenerators<State> for T
where
    T1: HasPredefinedGenerators<State> + 'static,
    T2: HasPredefinedGenerators<State> + 'static,
    T: Compose<Components = (T1, T2)> + 'static,
    T1::Generator: Clone,
    T2::Generator: Clone,
{
    type Generator = CompositionGenerator<T1::Generator, T2::Generator, Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        State: 'static,
    {
        let t1_generators = T1::generators();
        t1_generators.into_iter().flat_map(|g1| {
            T2::generators()
                .into_iter()
                .map(move |g2| CompositionGenerator::new(g1.clone(), g2.clone()))
        })
    }
}

#[derive(Debug)]
pub struct CompositionGenerator<G1, G2, T> {
    generator1: G1,
    generator2: G2,
    _phantom: PhantomData<fn() -> T>,
}

impl<G1, G2, T> CompositionGenerator<G1, G2, T> {
    pub const fn new(generator1: G1, generator2: G2) -> Self {
        Self {
            generator1,
            generator2,
            _phantom: PhantomData,
        }
    }
}

impl<G1, G2, T> Clone for CompositionGenerator<G1, G2, T>
where
    G1: Clone,
    G2: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.generator1.clone(), self.generator2.clone())
    }
}

impl<State, T, G1, G2> LspParamsGenerator<State> for CompositionGenerator<G1, G2, T>
where
    G1: LspParamsGenerator<State>,
    G2: LspParamsGenerator<State>,
    T: Compose<Components = (G1::Output, G2::Output)>,
{
    type Output = T;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let c1 = self.generator1.generate(state, input)?;
        let c2 = self.generator2.generate(state, input)?;
        let output = T::compose((c1, c2));
        Ok(output)
    }
}
