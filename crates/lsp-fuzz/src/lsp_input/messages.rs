use std::{borrow::Cow, marker::PhantomData, mem};

use derive_more::derive::{Deref, DerefMut};
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
use lsp_types::Uri;
use serde::{Deserialize, Deserializer, Serialize};
use tuple_list::{tuple_list, tuple_list_type};

use super::LspInput;
use crate::{
    lsp::{
        self, GeneratorsConfig,
        code_context::CodeContextRef,
        generation::registration::{
            append_diagnostic_messages, append_formatting_messages, append_hierarchy_messages,
            append_navigation_messages, append_symbol_messages, append_tracing_misc_messages,
            append_workspace_messages,
        },
        json_rpc::MessageId,
    },
    lsp_input::message_edit,
    macros::prop_mutator,
    mutators::SliceSwapMutator,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deref, DerefMut)]
pub struct LspMessageSequence {
    inner: Vec<lsp::LspMessage>,
}

impl<'de> Deserialize<'de> for LspMessageSequence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LspMessageSequenceRepr {
            inner: Vec<lsp::LspMessage>,
        }

        LspMessageSequenceRepr::deserialize(deserializer).map(|repr| Self { inner: repr.inner })
    }
}

#[derive(Debug)]
pub struct EnumMessages<'a> {
    next_id: usize,
    messages: <&'a Vec<lsp::LspMessage> as IntoIterator>::IntoIter,
}

impl<'a> Iterator for EnumMessages<'a> {
    type Item = (Option<MessageId>, &'a lsp::LspMessage);

    fn next(&mut self) -> Option<Self::Item> {
        match self.messages.next() {
            Some(msg) if msg.is_request() => {
                let new_id = self.next_id + 1;
                let id = mem::replace(&mut self.next_id, new_id);
                Some((Some(MessageId::Number(id)), msg))
            }
            Some(msg) if msg.is_notification() => Some((None, msg)),
            None => None,
            _ => unreachable!(),
        }
    }
}

impl LspMessageSequence {
    #[must_use]
    pub fn enumerate_messages(&self) -> EnumMessages<'_> {
        EnumMessages {
            next_id: 0,
            messages: self.inner.iter(),
        }
    }

    pub fn calibrate(&mut self, doc_uri: &Uri, input_edit: tree_sitter::InputEdit) {
        self.inner
            .iter_mut()
            .filter(|it| it.document().is_some_and(|it| &it.uri == doc_uri))
            .for_each(|message| message_edit::calibrate_message(message, input_edit));
    }
}

impl HasLen for LspMessageSequence {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

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
