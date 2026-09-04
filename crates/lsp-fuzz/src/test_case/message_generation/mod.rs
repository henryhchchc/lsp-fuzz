use std::{marker::Sized, ops::Deref, rc::Rc, result::Result};

use crate::test_case::LspInput;
use numeric::TabSizeGen;
use serde::{Deserialize, Serialize};

pub mod compositions;
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

pub trait HasGenerators<State> {
    type Generator: LspParamsGenerator<State, Output = Self>;

    fn generators(config: &GeneratorsConfig) -> impl IntoIterator<Item = Self::Generator>;
}

pub trait Compose {
    type Components;

    fn compose(components: Self::Components) -> Self;
}

impl<Head, Tail> Compose for (Head, Tail) {
    type Components = (Head, Tail);

    #[inline]
    fn compose(components: Self::Components) -> Self {
        components
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorsConfig {
    pub invalid_input: InvalidInputConfig,
    pub tab_size: TabSizeGen,
    pub awareness: AwarenessConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvalidInputConfig {
    pub ranges: bool,
    pub positions: bool,
    pub code_frequency: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwarenessConfig {
    pub grammar_ops: bool,
    pub context: bool,
    pub feedback_guidance: bool,
}

impl GeneratorsConfig {
    fn defaults() -> (InvalidInputConfig, TabSizeGen) {
        (
            InvalidInputConfig {
                ranges: true,
                positions: true,
                code_frequency: 0.1,
            },
            TabSizeGen {
                candidates: vec![0, 1, 2, 4, 8],
                rand_prob: 0.2,
            },
        )
    }

    #[must_use]
    pub fn full() -> Self {
        let (invalid_input, tab_size) = Self::defaults();
        Self {
            invalid_input,
            tab_size,
            awareness: AwarenessConfig {
                grammar_ops: true,
                context: true,
                feedback_guidance: true,
            },
        }
    }

    #[must_use]
    pub fn no_server_feedback() -> Self {
        let (invalid_input, tab_size) = Self::defaults();
        Self {
            invalid_input,
            tab_size,
            awareness: AwarenessConfig {
                grammar_ops: false,
                context: true,
                feedback_guidance: false,
            },
        }
    }

    #[must_use]
    pub fn no_context_awareness() -> Self {
        let (invalid_input, tab_size) = Self::defaults();
        Self {
            invalid_input,
            tab_size,
            awareness: AwarenessConfig {
                grammar_ops: true,
                context: false,
                feedback_guidance: false,
            },
        }
    }

    #[must_use]
    pub const fn use_context(&self) -> bool {
        self.awareness.context
    }

    #[must_use]
    pub const fn use_feedback_guidance(&self) -> bool {
        self.awareness.context && self.awareness.feedback_guidance
    }

    #[must_use]
    pub const fn use_grammar_ops(&self) -> bool {
        self.awareness.context && self.awareness.grammar_ops
    }

    #[must_use]
    pub const fn allow_invalid_positions(&self) -> bool {
        self.awareness.context && self.invalid_input.positions
    }

    #[must_use]
    pub const fn allow_invalid_ranges(&self) -> bool {
        self.awareness.context && self.invalid_input.ranges
    }
}

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
