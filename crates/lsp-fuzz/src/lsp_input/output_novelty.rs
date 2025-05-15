use std::{borrow::Cow, collections::HashSet};

use derive_more::Debug;
use fastbloom::BloomFilter;
use libafl::{
    HasMetadata,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
};
use libafl_bolts::{
    Named, SerdeAny,
    tuples::{Handle, Handled, MatchNameRef},
};
use lsp_types::{NumberOrString, notification::PublishDiagnostics};
use serde::{Deserialize, Serialize};

use super::LspInput;
use crate::{execution::responses::LspOutputObserver, utils::AflContext};

#[derive(Debug)]
pub struct OutputNoveltyFeedback {
    observer_handle: Handle<LspOutputObserver>,
}

impl OutputNoveltyFeedback {
    pub fn new(observer: &LspOutputObserver) -> Self {
        Self {
            observer_handle: observer.handle(),
        }
    }
}

impl Named for OutputNoveltyFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ResponseNoveltyFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for OutputNoveltyFeedback
where
    State: HasMetadata,
{
    fn init_state(&mut self, state: &mut State) -> Result<(), libafl::Error> {
        state.add_metadata(ObservedDiagnosticCodes::default());
        Ok(())
    }
}

impl<EM, OT, State> Feedback<EM, LspInput, OT, State> for OutputNoveltyFeedback
where
    State: HasMetadata,
    OT: MatchNameRef,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &LspInput,
        observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error> {
        let observed_diagnostic_codes = _state
            .metadata_mut::<ObservedDiagnosticCodes>()
            .afl_context("ObservedDiagnosticCodes not attached")?;

        let observer = observers
            .get(&self.observer_handle)
            .afl_context("ResponseObserver not attached")?;
        let messages = observer.captured_messages();
        let Some(diagnostics) = messages
            .iter()
            .filter_map(|msg| msg.extract_notification_param::<PublishDiagnostics>().ok())
            .next()
        else {
            return Ok(false);
        };

        let diag_types: HashSet<_> = diagnostics
            .diagnostics
            .into_iter()
            .map(|it| (it.code, it.source))
            .collect();

        let is_interesting = diag_types
            .iter()
            .any(|it| observed_diagnostic_codes.merge(it));

        Ok(is_interesting)
    }

    fn append_metadata(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _observers: &OT,
        _testcase: &mut libafl::corpus::Testcase<LspInput>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, SerdeAny)]
pub struct ObservedDiagnosticCodes {
    inner: BloomFilter,
}

impl Default for ObservedDiagnosticCodes {
    fn default() -> Self {
        Self {
            inner: BloomFilter::with_false_pos(0.001).expected_items(1_000_000),
        }
    }
}

impl ObservedDiagnosticCodes {
    pub fn merge(&mut self, code: &(Option<NumberOrString>, Option<String>)) -> bool {
        !self.inner.insert(&code)
    }
}
