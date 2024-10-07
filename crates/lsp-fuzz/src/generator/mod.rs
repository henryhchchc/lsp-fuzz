use libafl::prelude::Generator;

use crate::LspInput;

#[derive(Debug)]
pub struct LspInpuGenerator;

impl<S> Generator<LspInput, S> for LspInpuGenerator {
    fn generate(&mut self, state: &mut S) -> Result<LspInput, libafl::Error> {
        Ok(LspInput { bytes: Vec::new() })
    }
}
