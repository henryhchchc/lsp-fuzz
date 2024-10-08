use libafl::{prelude::Generator, state::HasRand};
use libafl_bolts::rands::Rand;

use crate::LspInput;

#[derive(Debug)]
pub struct LspInpuGenerator;

impl<S> Generator<LspInput, S> for LspInpuGenerator
where
    S: HasRand,
{
    fn generate(&mut self, state: &mut S) -> Result<LspInput, libafl::Error> {
        let byte = state.rand_mut().below(256) as u8;
        Ok(LspInput {
            bytes: vec![b'a', byte],
        })
    }
}
