use std::borrow::Cow;

use derive_more::Debug;
use derive_new::new as New;
use libafl::{
    corpus::{Corpus, CorpusId, Testcase},
    feedbacks::{Feedback, StateInitializer},
    state::{HasCorpus, HasExecutions, HasStartTime},
};
use libafl_bolts::{Named, current_time};

#[derive(Debug, New)]
pub struct TestCaseFileNameFeedback;

impl Named for TestCaseFileNameFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("TestCaseFileNameFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for TestCaseFileNameFeedback {}

impl<State, EM, I, OT> Feedback<EM, I, OT, State> for TestCaseFileNameFeedback
where
    State: HasExecutions + HasStartTime + HasCorpus<I>,
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
        testcase: &mut Testcase<I>,
    ) -> Result<(), libafl::Error> {
        let CorpusId(id) = state.corpus().peek_free_id();
        let time = (current_time() - *state.start_time()).as_secs();
        let exec = *state.executions();

        let file_name = format!("id_{id}_time_{time}_exec_{exec}");
        *testcase.filename_mut() = Some(file_name);
        Ok(())
    }
}
