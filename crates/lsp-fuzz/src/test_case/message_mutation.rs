use super::{
    LspInput,
    message_generation::{
        GeneratorsConfig,
        registration::{
            append_diagnostic_messages, append_formatting_messages, append_hierarchy_messages,
            append_navigation_messages, append_symbol_messages, append_tracing_misc_messages,
            append_workspace_messages,
        },
    },
};
use crate::{
    libafl_support::{mutators::SliceSwapMutator, prop_mutator},
    lsp,
};
use derive_new::new as New;
use libafl::{
    HasMetadata,
    mutators::{MutationResult, Mutator, MutatorsTuple},
    state::{HasCurrentTestcase, HasRand},
};
use libafl_bolts::{
    HasLen, Named,
    rands::Rand,
    tuples::{Merge, NamedTuple},
};
use std::{borrow::Cow, marker::PhantomData};
use tuple_list::{tuple_list, tuple_list_type};

#[derive(Debug, New)]
pub struct DropRandomMessage<State> {
    _state: PhantomData<State>,
}

impl<State> Named for DropRandomMessage<State> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("DropRandomMessage");
        &NAME
    }
}

impl<State> Mutator<LspInput, State> for DropRandomMessage<State>
where
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let rand = state.rand_mut();
        if let Some(index) = rand.choose(0..input.messages.len()) {
            input.messages.remove(index);
            Ok(MutationResult::Mutated)
        } else {
            Ok(MutationResult::Skipped)
        }
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

prop_mutator!(pub impl MessagesMutator for LspInput::messages type Vec<lsp::LspMessage>);

pub type SwapRequests<State> = MessagesMutator<SliceSwapMutator<lsp::LspMessage, State>>;

#[must_use]
pub fn message_mutations<State>(
    config: &GeneratorsConfig,
) -> impl MutatorsTuple<LspInput, State> + NamedTuple + use<State>
where
    State: HasRand + HasMetadata + HasCurrentTestcase<LspInput> + 'static,
{
    let swap = tuple_list![SwapRequests::new(SliceSwapMutator::new())];
    append_navigation_messages(config)
        .merge(append_symbol_messages(config))
        .merge(append_formatting_messages(config))
        .merge(append_hierarchy_messages(config))
        .merge(append_workspace_messages(config))
        .merge(append_diagnostic_messages(config))
        .merge(append_tracing_misc_messages(config))
        .merge(swap)
        .merge(message_reductions())
}

#[must_use]
pub fn message_reductions<State>() -> tuple_list_type![DropRandomMessage<State>]
where
    State: HasRand,
{
    tuple_list![DropRandomMessage::new()]
}
