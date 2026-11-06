use super::message_edit;
use crate::lsp::{self, code_context::CodeContextRef, json_rpc::MessageId};
use derive_more::derive::{Deref, DerefMut};
use libafl_bolts::HasLen;
use lsp_types::Uri;
use serde::{Deserialize, Deserializer, Serialize};
use std::mem;

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
