use std::borrow::Cow;

use derive_more::Debug;
use libafl::{
    HasMetadata,
    corpus::{Corpus, Testcase},
    events::EventFirer,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    state::{HasCorpus, HasExecutions},
};
use libafl_bolts::{
    Named,
    tuples::{Handle, Handled, MatchNameRef},
};
use tracing::warn;

use super::LspInput;
use crate::{
    execution::responses::LspOutputObserver,
    lsp_input::server_response::collector::collect_response_info, utils::AflContext,
};

mod collector;
pub mod matching;
pub mod metadata;

#[derive(Debug)]
pub struct LspResponseFeedback {
    observer_handle: Handle<LspOutputObserver>,
}

impl LspResponseFeedback {
    #[must_use]
    pub fn new(observer: &LspOutputObserver) -> Self {
        Self {
            observer_handle: observer.handle(),
        }
    }
}

impl Named for LspResponseFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ResponseNoveltyFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for LspResponseFeedback where State: HasMetadata {}

impl<EM, Observers, State> Feedback<EM, LspInput, Observers, State> for LspResponseFeedback
where
    State: HasMetadata + HasExecutions + HasCorpus<LspInput>,
    Observers: MatchNameRef,
    EM: EventFirer<LspInput, State>,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &LspInput,
        _observers: &Observers,
        _exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        observers: &Observers,
        testcase: &mut Testcase<LspInput>,
    ) -> Result<(), libafl::Error> {
        state
            .corpus()
            .load_input_into(testcase)
            .afl_context("Loading input to the test case")?;
        let input = testcase
            .input()
            .as_ref()
            .expect("We loaded the input just now.");

        let response_observer = observers
            .get(&self.observer_handle)
            .afl_context("LspResponseObserver not attached")?;
        let received_messages = response_observer.captured_messages();
        let Ok(matching) = matching::RequestResponseMatching::match_messages(
            input.messages.iter(),
            received_messages.iter(),
        ) else {
            warn!("Failed to match messages");
            return Ok(());
        };

        let response_info = collect_response_info(matching);
        testcase.add_metadata(response_info);
        Ok(())
    }
}
