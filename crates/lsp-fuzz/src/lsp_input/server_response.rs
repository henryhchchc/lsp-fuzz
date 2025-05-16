use std::{borrow::Cow, collections::HashSet};

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
use lsp_types::notification::PublishDiagnostics;
use metadata::{Diagnostic, LspResponseInfo};
use tracing::warn;

use super::LspInput;
use crate::{execution::responses::LspOutputObserver, utils::AflContext};

pub mod metadata;

pub mod matching;

#[derive(Debug)]
pub struct LspResponseFeedback {
    observer_handle: Handle<LspOutputObserver>,
}

impl LspResponseFeedback {
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

impl<EM, OT, State> Feedback<EM, LspInput, OT, State> for LspResponseFeedback
where
    State: HasMetadata + HasExecutions + HasCorpus<LspInput>,
    OT: MatchNameRef,
    EM: EventFirer<LspInput, State>,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &LspInput,
        _observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        observers: &OT,
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

        let mut diagnostics = HashSet::new();

        for pub_diag in matching.find_notifications::<PublishDiagnostics>() {
            let uri = LspInput::lift_uri(&pub_diag.uri);
            for diag_item in pub_diag.diagnostics.iter() {
                let diag = Diagnostic {
                    uri: uri.as_ref().to_owned(),
                    range: diag_item.range,
                };
                diagnostics.insert(diag);
            }
        }

        let response_info = LspResponseInfo { diagnostics };
        testcase.add_metadata(response_info);
        Ok(())
    }
}
