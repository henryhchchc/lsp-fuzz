use std::borrow::Cow;

use derive_new::new as New;
use libafl::{
    HasMetadata,
    corpus::Corpus,
    feedbacks::{Feedback, NautilusChunksMetadata, StateInitializer},
    generators::NautilusContext,
    inputs::{Input, NautilusTargetBytesConverter, TargetBytesConverter, nautilus::NautilusInput},
    state::HasCorpus,
};
use libafl_bolts::{HasLen, Named};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct BaselineInput {
    messages: Vec<NautilusInput>,
}

impl HasLen for BaselineInput {
    fn len(&self) -> usize {
        self.messages.iter().map(|it| it.len()).sum()
    }
}

impl Input for BaselineInput {}

#[derive(Debug, New)]
pub struct BaselineByteConverter<'a> {
    inner: NautilusTargetBytesConverter<'a>,
}

impl TargetBytesConverter<BaselineInput> for BaselineByteConverter<'_> {
    fn to_target_bytes<'a>(
        &mut self,
        input: &'a BaselineInput,
    ) -> libafl_bolts::ownedref::OwnedSlice<'a, u8> {
        let mut bytes = Vec::new();
        for message in input.messages.iter() {
            let message_bytes = self.inner.to_target_bytes(message);
            let header = format!("Content-Length: {}\r\n\r\n", message_bytes.len());
            bytes.extend(header.into_bytes());
            bytes.extend(message_bytes.to_vec());
        }
        bytes.into()
    }
}

#[derive(Debug, New)]
pub struct BaselineGrammarFeedback<'a> {
    context: &'a NautilusContext,
}

impl Named for BaselineGrammarFeedback<'_> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        const NAME: Cow<'static, str> = Cow::Borrowed("BaselineGrammarFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for BaselineGrammarFeedback<'_> {}

impl<'a, State, EM, OBS> Feedback<EM, BaselineInput, OBS, State> for BaselineGrammarFeedback<'a>
where
    State: HasMetadata + HasCorpus<BaselineInput>,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &BaselineInput,
        _observers: &OBS,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        _observers: &OBS,
        testcase: &mut libafl::corpus::Testcase<BaselineInput>,
    ) -> Result<(), libafl::Error> {
        state.corpus().load_input_into(testcase)?;
        let input = testcase.input().as_ref().unwrap().clone();
        let meta = state
            .metadata_map_mut()
            .get_mut::<NautilusChunksMetadata>()
            .expect("NautilusChunksMetadata not in the state");
        for msg in input.messages {
            meta.cks.add_tree(msg.tree().to_owned(), &self.context.ctx);
        }
        Ok(())
    }
}
