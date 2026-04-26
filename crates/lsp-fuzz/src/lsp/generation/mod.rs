use std::{marker::Sized, ops::Deref, rc::Rc, result::Result};

use super::HasGenerators;
use crate::lsp_input::LspInput;
pub mod containers;
pub mod core;
pub mod defaults;
pub mod doc;
pub mod doc_range;
pub mod numeric;
pub mod position;
pub(crate) mod position_selectors;
pub mod registration;
pub mod server_feedback;
pub mod string;

pub use core::{
    combinators::{
        DefaultGenerator, FallbackGenerator, OneOfGenerator, OptionGenerator,
        ParamFragmentGenerator,
    },
    composition::CompositionGenerator,
    consts::ConstGenerator,
    registry::{GeneratorBag, WeightedGeneratorList},
};

pub type DynGenerator<State, T> = Rc<dyn LspParamsGenerator<State, Output = T>>;

#[must_use]
pub fn boxed_generator<State, T, G>(generator: G) -> DynGenerator<State, T>
where
    G: LspParamsGenerator<State, Output = T> + 'static,
{
    Rc::new(generator)
}

pub trait LspParamsGenerator<State> {
    type Output;

    /// Produces parameters for an LSP message from the current state and input.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError`] when generation fails or produces no value.
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
