use std::{borrow::Cow, marker::PhantomData};

use libafl::{
    executors::ExitKind,
    feedback_or,
    feedbacks::{
        CombinedFeedback, DifferentIsNovel, Feedback, FeedbackFactory, HasObserverHandle,
        LogicEagerOr, MapFeedback, MaxMapFeedback, MaxReducer, StateInitializer,
    },
    observers::{CanTrack, MapObserver},
    state::State,
};
use libafl_bolts::{
    tuples::{Handle, Handled, MatchName, MatchNameRef},
    Named,
};

/// A feedback factory for ensuring that the maps for minimized inputs are the same
#[derive(Debug, Clone)]
pub struct MinimizationFeedbackFactory<C, M> {
    map_ref: Handle<C>,
    phantom: PhantomData<(C, M)>,
}

impl<C, M> MinimizationFeedbackFactory<C, M>
where
    M: MapObserver,
    C: AsRef<M> + Handled,
{
    /// Creates a new map equality feedback for the given observer
    pub fn new(obs: &C) -> Self {
        Self {
            map_ref: obs.handle(),
            phantom: PhantomData,
        }
    }
}

impl<C, M> HasObserverHandle for MinimizationFeedbackFactory<C, M> {
    type Observer = C;

    fn observer_handle(&self) -> &Handle<C> {
        &self.map_ref
    }
}

type MinimizationFeedback<C, M> = CombinedFeedback<
    MapEqualityFeedback<C, M>,
    MapFeedback<C, DifferentIsNovel, M, MaxReducer>,
    LogicEagerOr,
>;

impl<C, M, OT> FeedbackFactory<MinimizationFeedback<C, M>, OT> for MinimizationFeedbackFactory<C, M>
where
    M: MapObserver,
    C: AsRef<M> + CanTrack + Named,
    OT: MatchNameRef,
{
    fn create_feedback(&self, observers: &OT) -> MinimizationFeedback<C, M> {
        let obs = observers
            .get(self.observer_handle())
            .expect("Should have been provided valid observer name.");
        let map_eq_feedback = MapEqualityFeedback {
            name: Cow::from("MapEq"),
            map_ref: obs.handle(),
            orig_hash: obs.as_ref().hash_simple(),
            phantom: PhantomData,
        };
        feedback_or!(map_eq_feedback, MaxMapFeedback::new(obs))
    }
}

/// A feedback which checks if the hash of the currently observed map is equal to the original hash
/// provided
#[derive(Clone, Debug)]
pub struct MapEqualityFeedback<C, M> {
    name: Cow<'static, str>,
    map_ref: Handle<C>,
    orig_hash: u64,
    phantom: PhantomData<M>,
}

impl<C, M> Named for MapEqualityFeedback<C, M> {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<C, M> HasObserverHandle for MapEqualityFeedback<C, M> {
    type Observer = C;

    fn observer_handle(&self) -> &Handle<Self::Observer> {
        &self.map_ref
    }
}

impl<C, M, S> StateInitializer<S> for MapEqualityFeedback<C, M> {}

impl<C, EM, I, M, OT, S> Feedback<EM, I, OT, S> for MapEqualityFeedback<C, M>
where
    M: MapObserver,
    C: AsRef<M>,
    S: State,
    OT: MatchName,
{
    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &I,
        observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error> {
        let obs = observers
            .get(self.observer_handle())
            .expect("Should have been provided valid observer name.");
        let res = obs.as_ref().hash_simple() == self.orig_hash;
        Ok(res)
    }

    fn append_metadata(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _observers: &OT,
        _testcase: &mut libafl::corpus::Testcase<I>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn discard_metadata(&mut self, _state: &mut S, _input: &I) -> Result<(), libafl::Error> {
        Ok(())
    }
}
