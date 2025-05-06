use std::{borrow::Cow, num::NonZeroUsize};

use derive_new::new as New;
use libafl::{
    generators::Generator,
    inputs::{Input, InputToBytes},
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{HasLen, Named, rands::Rand};
use serde::{Deserialize, Serialize};

const MAX_MESSAGES: usize = 20;

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct BaselineInput<Message> {
    messages: Vec<Message>,
}

impl<Message> BaselineInput<Message> {
    pub fn messages(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }

    pub fn messages_mut(&mut self) -> impl Iterator<Item = &mut Message> {
        self.messages.iter_mut()
    }
}

impl<Message: HasLen> HasLen for BaselineInput<Message> {
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

impl<Message: Input> Input for BaselineInput<Message> {}

#[derive(Debug, New)]
pub struct BaselineByteConverter<MessageConverter> {
    inner: MessageConverter,
}

impl<Message, Converter> InputToBytes<BaselineInput<Message>> for BaselineByteConverter<Converter>
where
    Converter: InputToBytes<Message>,
{
    fn to_bytes<'a>(
        &mut self,
        input: &'a BaselineInput<Message>,
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

impl<State, Inner, Message> Mutator<BaselineInput<Message>, State> for BaselineGrammarMutator<Inner>
where
    State: HasRand,
    Inner: Mutator<Message, State>,
    Message: Clone,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut BaselineInput<Message>,
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

impl<State, Inner, Message> Mutator<BaselineInput<Message>, State>
    for BaselineSequenceMutator<Inner>
where
    State: HasRand,
    Inner: Generator<Message, State>,
    Message: Clone,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut BaselineInput<Message>,
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

impl<MsgGen, State, Message> Generator<BaselineInput<Message>, State>
    for BaselineInputGenerator<MsgGen>
where
    MsgGen: Generator<Message, State>,
{
    fn generate(&mut self, state: &mut State) -> Result<BaselineInput<Message>, libafl::Error> {
        let msg = self.inner.generate(state)?;
        Ok(BaselineInput {
            messages: vec![msg],
        })
    }
}
