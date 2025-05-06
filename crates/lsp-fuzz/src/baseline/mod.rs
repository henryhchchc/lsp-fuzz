use std::{borrow::Cow, num::NonZeroUsize};

use derive_new::new as New;
use libafl::{
    HasMetadata,
    corpus::Corpus,
    feedbacks::{Feedback, NautilusChunksMetadata, StateInitializer},
    generators::{Generator, NautilusContext},
    inputs::{Input, InputToBytes, NautilusBytesConverter, nautilus::NautilusInput},
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasRand},
};
use libafl_bolts::{HasLen, Named, rands::Rand};
use serde::{Deserialize, Serialize};

const MAX_MESSAGES: usize = 20;

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct BaselineInput {
    messages: Vec<NautilusInput>,
}

impl HasLen for BaselineInput {
    fn len(&self) -> usize {
        self.messages
            .iter()
            .map(|it| {
                let content_len = it.len();
                let header_len = format!("Content-Length: {content_len}\r\n\r\n").len();
                content_len + header_len
            })
            .sum()
    }
}

impl Input for BaselineInput {}

#[derive(Debug, New)]
pub struct BaselineByteConverter<'a> {
    inner: NautilusBytesConverter<'a>,
}

impl InputToBytes<BaselineInput> for BaselineByteConverter<'_> {
    fn to_bytes<'a>(
        &mut self,
        input: &'a BaselineInput,
    ) -> libafl_bolts::ownedref::OwnedSlice<'a, u8> {
        let mut bytes = Vec::new();
        for message in input.messages.iter() {
            let message_bytes = self.inner.to_bytes(message);
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

#[derive(Debug, New, Clone)]
pub struct BaselineGrammarMutator<MsgMut> {
    message_mutator: MsgMut,
}

impl<MsgMut> Named for BaselineGrammarMutator<MsgMut> {
    fn name(&self) -> &Cow<'static, str> {
        const NAME: Cow<'static, str> = Cow::Borrowed("BaselineGrammarMutator");
        &NAME
    }
}

impl<State, MsgMut> Mutator<BaselineInput, State> for BaselineGrammarMutator<MsgMut>
where
    State: HasRand,
    MsgMut: Mutator<NautilusInput, State>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut BaselineInput,
    ) -> Result<MutationResult, libafl::Error> {
        let rand = state.rand_mut();
        let Some(message) = rand.choose(input.messages.iter_mut()) else {
            return Ok(MutationResult::Skipped);
        };
        self.message_mutator.mutate(state, message)
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

#[derive(Debug, New)]
pub struct BaselineSequenceMutator<MsgGen> {
    message_generator: MsgGen,
}

impl<MsgGen> Named for BaselineSequenceMutator<MsgGen> {
    fn name(&self) -> &Cow<'static, str> {
        const NAME: Cow<'static, str> = Cow::Borrowed("BaselineSequenceMutator");
        &NAME
    }
}

impl<State, MsgGen> Mutator<BaselineInput, State> for BaselineSequenceMutator<MsgGen>
where
    State: HasRand,
    MsgGen: Generator<NautilusInput, State>,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut BaselineInput,
    ) -> Result<MutationResult, libafl::Error> {
        let rand = state.rand_mut();

        // Decide what operation to perform
        match rand.below(NonZeroUsize::new(4).unwrap()) {
            0 => {
                // Add a message if we're below the maximum
                if input.messages.len() < MAX_MESSAGES {
                    let new_message = self.message_generator.generate(state)?;
                    input.messages.push(new_message);
                    Ok(MutationResult::Mutated)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            1 => {
                // Remove a message if we have more than one
                if !input.messages.is_empty() {
                    let idx = rand.below(input.messages.len().try_into().unwrap());
                    input.messages.remove(idx);
                    Ok(MutationResult::Mutated)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            2 => {
                // Swap two messages if we have more than one
                if input.messages.len() > 1 {
                    let bound = input.messages.len().try_into().unwrap();
                    let idx1 = rand.below(bound);
                    let idx2 = rand.below(bound);
                    input.messages.swap(idx1, idx2);
                    Ok(MutationResult::Mutated)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            3 => {
                // Duplicate a message if we're below the maximum
                if !input.messages.is_empty() && input.messages.len() < MAX_MESSAGES {
                    let message = rand
                        .choose(input.messages.iter())
                        .expect("We checked that is it not empty");
                    input.messages.push(message.clone());
                    Ok(MutationResult::Mutated)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            _ => unreachable!(),
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

#[derive(Debug, New)]
pub struct BaselineInputGenerator<MsgGen> {
    inner: MsgGen,
}

impl<MsgGen, State> Generator<BaselineInput, State> for BaselineInputGenerator<MsgGen>
where
    MsgGen: Generator<NautilusInput, State>,
{
    fn generate(&mut self, state: &mut State) -> Result<BaselineInput, libafl::Error> {
        let msg = self.inner.generate(state)?;
        Ok(BaselineInput {
            messages: vec![msg],
        })
    }
}
