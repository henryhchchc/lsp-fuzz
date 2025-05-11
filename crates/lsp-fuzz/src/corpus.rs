use std::{borrow::Cow, time::Duration};

use derive_more::Debug;
use derive_new::new as New;
use libafl::{
    HasMetadata,
    feedbacks::{Feedback, StateInitializer},
    state::{HasExecutions, HasStartTime},
};
use libafl_bolts::{Named, SerdeAny, current_time};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, SerdeAny, Clone)]
pub struct GeneratedStats {
    generated_time: Duration,
    generated_exec: u64,
}

#[derive(Debug, New)]
pub struct GeneratedStatsFeedback;

impl Named for GeneratedStatsFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("GeneratedStatsFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for GeneratedStatsFeedback {}

impl<State, EM, I, OT> Feedback<EM, I, OT, State> for GeneratedStatsFeedback
where
    State: HasExecutions + HasStartTime,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &I,
        _observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        _observers: &OT,
        testcase: &mut libafl::corpus::Testcase<I>,
    ) -> Result<(), libafl::Error> {
        let metadata = GeneratedStats {
            generated_exec: *state.executions(),
            generated_time: current_time() - *state.start_time(),
        };
        testcase.add_metadata(metadata);
        Ok(())
    }
}
