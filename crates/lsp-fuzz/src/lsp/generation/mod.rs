use std::{marker::Sized, ops::Deref, result::Result};

use super::HasPredefinedGenerators;
use crate::lsp_input::LspInput;
pub mod composition;
pub mod consts;
pub mod doc;
pub mod doc_range;
pub mod meta;
pub mod numeric;
pub mod position;
pub mod server_feedback;
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
