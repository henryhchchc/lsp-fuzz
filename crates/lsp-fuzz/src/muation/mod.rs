use std::borrow::Cow;

use libafl::{
    inputs::UsesInput,
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand, State},
    HasMetadata,
};
use libafl_bolts::Named;

use crate::inputs::LspInput;

#[derive(Debug)]
pub struct LspInputMutator<M> {
    inner_muatator: M,
}

impl<M> LspInputMutator<M> {
    pub fn new(inner_muatator: M) -> Self {
        Self { inner_muatator }
    }
}

impl<M> Named for LspInputMutator<M> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &Cow::Borrowed("LspInputMutator")
    }
}

impl<M, S> Mutator<LspInput, S> for LspInputMutator<M>
where
    M: Mutator<LspInput, S>,
    S: State + UsesInput<Input = LspInput> + HasMetadata + HasCorpus + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        self.inner_muatator.mutate(state, input)
    }
}
