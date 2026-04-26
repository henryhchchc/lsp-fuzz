mod diagnostics;
mod formatting;
mod hierarchy;
mod navigation;
mod symbols;
mod tracing_misc;
mod workspace;

use std::{any::type_name, borrow::Cow, fmt::Debug};

use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{Named, rands::Rand};

use crate::{
    lsp::{
        GeneratorsConfig, HasGenerators, LspMessage, LspMessageMeta, MessageParam,
        generation::LspParamsGenerator,
    },
    lsp_input::LspInput,
};

pub use diagnostics::append_diagnostic_messages;
pub use formatting::append_formatting_messages;
pub use hierarchy::append_hierarchy_messages;
pub use navigation::append_navigation_messages;
pub use symbols::append_symbol_messages;
pub use tracing_misc::append_tracing_misc_messages;
pub use workspace::append_workspace_messages;

pub struct AppendMessage<M, State>
where
    M: LspMessageMeta,
    M::Params: HasGenerators<State>,
{
    name: Cow<'static, str>,
    generators: Vec<<M::Params as HasGenerators<State>>::Generator>,
}

impl<M: LspMessageMeta, State> Debug for AppendMessage<M, State>
where
    M::Params: HasGenerators<State>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let generators_desc = format!(
            "{} {}",
            self.generators.len(),
            type_name::<<M::Params as HasGenerators<State>>::Generator>()
        );
        f.debug_struct("AppendRandomlyGeneratedMessage")
            .field("name", &self.name)
            .field("generators", &generators_desc)
            .finish()
    }
}

pub const MAX_MESSAGES: usize = 20;

impl<M, State> AppendMessage<M, State>
where
    M: LspMessageMeta,
    M::Params: HasGenerators<State>,
{
    /// Creates an append mutator from the predefined generators for `M`.
    ///
    /// # Panics
    ///
    /// Panics if `M::Params::generators(config)` returns no generators.
    #[must_use]
    pub fn with_predefined(config: &GeneratorsConfig) -> Self {
        let name = Cow::Owned(format!("AppendRandomlyGenerated {}", M::METHOD));
        let generators: Vec<_> = M::Params::generators(config).into_iter().collect();
        assert!(!generators.is_empty(), "No generators for {}", M::METHOD);
        Self { name, generators }
    }
}

impl<M, State> Named for AppendMessage<M, State>
where
    M: LspMessageMeta,
    M::Params: HasGenerators<State>,
{
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<M, State> Mutator<LspInput, State> for AppendMessage<M, State>
where
    State: HasRand,
    M: LspMessageMeta,
    M::Params: HasGenerators<State> + MessageParam<M>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some(generator) = state.rand_mut().choose(&self.generators) else {
            return Ok(MutationResult::Skipped);
        };
        let params = match generator.generate(state, input) {
            Ok(params) => params,
            Err(crate::lsp::generation::GenerationError::NothingGenerated) => {
                return Ok(MutationResult::Skipped);
            }
            Err(crate::lsp::generation::GenerationError::Error(err)) => return Err(err),
        };
        let message = LspMessage::from_params::<M>(params);
        if input.messages.len() >= MAX_MESSAGES {
            let being_replaced = state.rand_mut().choose(input.messages.iter_mut()).expect(
                "There must be at least one message in the input when entering this branch",
            );
            *being_replaced = message;
        } else {
            input.messages.push(message);
        }
        Ok(MutationResult::Mutated)
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}
