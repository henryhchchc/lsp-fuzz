use std::{
    borrow::Cow,
    cell::{Ref, RefCell, RefMut},
    collections::VecDeque,
};

use derive_more::Debug;
use derive_new::new as New;
use libafl::{
    HasMetadata,
    corpus::{Corpus, CorpusId, HasTestcase, Testcase},
    feedbacks::{Feedback, StateInitializer},
    state::{HasCorpus, HasExecutions, HasStartTime},
};
use libafl_bolts::{Named, SerdeAny, current_time};
use serde::{Deserialize, Serialize};

use crate::utils::AflContext;

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

#[derive(Debug, Serialize, Deserialize, SerdeAny, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct CacheCorpusId(CorpusId);

#[derive(Debug, Serialize, Deserialize)]
pub struct ProperCachedCorpus<Inner> {
    inner: Inner,
    cache_size: usize,
    cached_test_case_ids: RefCell<VecDeque<CorpusId>>,
}

impl<Inner> ProperCachedCorpus<Inner> {
    pub fn new(inner: Inner, cache_size: usize) -> Self {
        let cached_test_case_ids = RefCell::new(VecDeque::with_capacity(cache_size));
        Self {
            inner,
            cache_size,
            cached_test_case_ids,
        }
    }

    fn fun_name<I>(&self, testcase: &mut Testcase<I>) -> Result<(), libafl::Error>
    where
        Inner: Corpus<I>,
    {
        let &CacheCorpusId(id) = testcase.metadata().afl_context("Getting id metadata")?;
        let mut borrowed_num = 0;
        while self.cached_test_case_ids.borrow().len() >= self.cache_size {
            let to_evic = self.cached_test_case_ids.borrow_mut().pop_front().unwrap();

            if let Ok(mut borrowed) = self.inner.get_from_all(to_evic)?.try_borrow_mut() {
                *borrowed.input_mut() = None;
            } else {
                self.cached_test_case_ids.borrow_mut().push_back(to_evic);
                borrowed_num += 1;
                if self.cache_size == borrowed_num {
                    break;
                }
            }
        }
        self.cached_test_case_ids.borrow_mut().push_back(id);
        Ok(())
    }
}

impl<Inner, I> Corpus<I> for ProperCachedCorpus<Inner>
where
    Inner: Corpus<I>,
{
    fn count(&self) -> usize {
        self.inner.count()
    }

    fn count_disabled(&self) -> usize {
        self.inner.count_disabled()
    }

    fn count_all(&self) -> usize {
        self.inner.count_all()
    }

    fn add(&mut self, testcase: Testcase<I>) -> Result<CorpusId, libafl::Error> {
        let id = self.inner.add(testcase)?;
        let mut test_case = self.get(id).expect("We added it just now").borrow_mut();
        test_case.add_metadata(CacheCorpusId(id));
        Ok(id)
    }

    fn add_disabled(&mut self, testcase: Testcase<I>) -> Result<CorpusId, libafl::Error> {
        let id = self.inner.add_disabled(testcase)?;
        let mut test_case = self
            .get_from_all(id)
            .expect("We added it just now")
            .borrow_mut();
        test_case.add_metadata(CacheCorpusId(id));
        Ok(id)
    }

    fn replace(
        &mut self,
        id: CorpusId,
        mut testcase: Testcase<I>,
    ) -> Result<Testcase<I>, libafl::Error> {
        testcase.add_metadata(CacheCorpusId(id));
        self.inner.replace(id, testcase)
    }

    fn remove(&mut self, id: CorpusId) -> Result<Testcase<I>, libafl::Error> {
        self.inner.remove(id)
    }

    fn get(&self, id: CorpusId) -> Result<&RefCell<Testcase<I>>, libafl::Error> {
        self.inner.get(id)
    }

    fn get_from_all(&self, id: CorpusId) -> Result<&RefCell<Testcase<I>>, libafl::Error> {
        self.inner.get_from_all(id)
    }

    fn current(&self) -> &Option<CorpusId> {
        self.inner.current()
    }

    fn current_mut(&mut self) -> &mut Option<CorpusId> {
        self.inner.current_mut()
    }

    fn next(&self, id: CorpusId) -> Option<CorpusId> {
        self.inner.next(id)
    }

    fn peek_free_id(&self) -> CorpusId {
        self.inner.peek_free_id()
    }

    fn prev(&self, id: CorpusId) -> Option<CorpusId> {
        self.inner.prev(id)
    }

    fn first(&self) -> Option<CorpusId> {
        self.inner.first()
    }

    fn last(&self) -> Option<CorpusId> {
        self.inner.last()
    }

    fn nth_from_all(&self, nth: usize) -> CorpusId {
        self.inner.nth_from_all(nth)
    }

    fn load_input_into(&self, testcase: &mut Testcase<I>) -> Result<(), libafl::Error> {
        if testcase.input().is_none() {
            self.inner.load_input_into(testcase)?;
            self.fun_name(testcase)?;
        }
        Ok(())
    }

    fn store_input_from(&self, testcase: &Testcase<I>) -> Result<(), libafl::Error> {
        self.inner.store_input_from(testcase)
    }
}

impl<Inner, I> HasTestcase<I> for ProperCachedCorpus<Inner>
where
    Inner: HasTestcase<I>,
{
    fn testcase(&self, id: CorpusId) -> Result<Ref<'_, Testcase<I>>, libafl::Error> {
        self.inner.testcase(id)
    }

    fn testcase_mut(&self, id: CorpusId) -> Result<RefMut<'_, Testcase<I>>, libafl::Error> {
        self.inner.testcase_mut(id)
    }
}
