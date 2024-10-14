
use libafl::{generators::Generator, state::HasRand};

use crate::inputs::LspInput;

#[derive(Debug)]
pub struct LspInpuGenerator;

impl<S> Generator<LspInput, S> for LspInpuGenerator
where
    S: HasRand,
{
    fn generate(&mut self, _state: &mut S) -> Result<LspInput, libafl::Error> {
        Ok(LspInput::default())
    }
}
