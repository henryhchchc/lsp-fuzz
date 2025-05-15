use std::{
    borrow::Cow,
    io::{self, BufRead},
};

use libafl::observers::Observer;
use libafl_bolts::Named;
use serde::{Deserialize, Serialize};

use crate::lsp::json_rpc::JsonRPCMessage;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ResponsesObserver {
    captured_messages: Vec<JsonRPCMessage>,
}

impl Named for ResponsesObserver {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ResponsesObserver");
        &NAME
    }
}

impl ResponsesObserver {
    pub fn new() -> Self {
        Self {
            captured_messages: Vec::new(),
        }
    }

    pub fn captured_messages(&self) -> &[JsonRPCMessage] {
        &self.captured_messages
    }

    pub fn capture_stdout_content<R: BufRead>(&mut self, mut reader: R) -> io::Result<()> {
        while let Ok(message) = JsonRPCMessage::read_lsp_payload(&mut reader) {
            self.captured_messages.push(message);
        }
        Ok(())
    }
}

impl<I, State> Observer<I, State> for ResponsesObserver {
    fn pre_exec(&mut self, _state: &mut State, _input: &I) -> Result<(), libafl::Error> {
        self.captured_messages.clear();
        Ok(())
    }
}
