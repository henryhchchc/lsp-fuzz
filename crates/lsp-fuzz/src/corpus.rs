use std::borrow::Cow;

use corpus_kind::{CORPUS, SOLUTION};
use derive_more::Debug;
use derive_new::new as New;
use libafl::{
    corpus::{Corpus, CorpusId, Testcase},
    feedbacks::{Feedback, StateInitializer},
    state::{HasCorpus, HasExecutions, HasSolutions, HasStartTime},
};
use libafl_bolts::{Named, SerdeAny, current_time};
use serde::{Deserialize, Serialize};

#[derive(Debug, New)]
pub struct TestCaseFileNameFeedback<const KIND: bool>;

pub mod corpus_kind {
    pub const CORPUS: bool = true;
    pub const SOLUTION: bool = false;
}

impl<const KIND: bool> Named for TestCaseFileNameFeedback<KIND> {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("TestCaseFileNameFeedback");
        &NAME
    }
}

impl<const KIND: bool, State> StateInitializer<State> for TestCaseFileNameFeedback<KIND> {}

impl<State, EM, I, Observers> Feedback<EM, I, Observers, State> for TestCaseFileNameFeedback<CORPUS>
where
    State: HasExecutions + HasStartTime + HasCorpus<I>,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &I,
        _observers: &Observers,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        _observers: &Observers,
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

impl<State, EM, I, Observers> Feedback<EM, I, Observers, State>
    for TestCaseFileNameFeedback<SOLUTION>
where
    State: HasExecutions + HasStartTime + HasSolutions<I>,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &I,
        _observers: &Observers,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        _observers: &Observers,
        testcase: &mut Testcase<I>,
    ) -> Result<(), libafl::Error> {
        let CorpusId(id) = state.solutions().peek_free_id();
        let time = (current_time() - *state.start_time()).as_secs();
        let exec = *state.executions();

        let file_name = format!("id_{id}_time_{time}_exec_{exec}");
        *testcase.filename_mut() = Some(file_name);
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, SerdeAny, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct CacheCorpusId(CorpusId);
