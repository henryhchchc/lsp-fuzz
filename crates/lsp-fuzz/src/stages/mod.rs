use std::{env::temp_dir, marker::PhantomData, thread};

use libafl::{
    corpus::Corpus,
    events::{EventFirer, LogSeverity},
    executors::HasObservers,
    inputs::UsesInput,
    observers::MapObserver,
    stages::Stage,
    state::{HasCorpus, HasExecutions, State, UsesState},
};
use libafl_bolts::{
    tuples::{Handle, Handled, MatchNameRef},
    Named,
};

#[derive(Debug)]
pub struct CoverageStage<O, S, M> {
    edge_observer_handle: Handle<O>,
    last_corpus_count: usize,
    _phantom: PhantomData<(S, M)>,
}

impl<O, S, I, M> UsesState for CoverageStage<O, S, M>
where
    S: UsesInput<Input = I> + State,
{
    type State = S;
}

impl<O, S, M> CoverageStage<O, S, M> {
    pub fn new(edge_observer: &O) -> Self
    where
        M: MapObserver,
        O: AsRef<M> + Named,
    {
        Self {
            edge_observer_handle: edge_observer.handle(),
            last_corpus_count: 0,
            _phantom: PhantomData,
        }
    }
}

impl<E, EM, Z, S, M, O> Stage<E, EM, Z> for CoverageStage<O, S, M>
where
    S: State + HasCorpus,
    Self: UsesState<State = S>,
    E: UsesState<State = S> + HasObservers,
    <E as HasObservers>::Observers: MatchNameRef,
    EM: UsesState<State = S> + EventFirer,
    Z: UsesState<State = S>,
    M: MapObserver,
    O: AsRef<M> + Named,
{
    fn should_restart(&mut self, state: &mut S) -> Result<bool, libafl::Error> {
        Ok(self.last_corpus_count < state.corpus().count())
    }

    fn clear_progress(&mut self, _state: &mut S) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        executor: &mut E,
        state: &mut S,
        manager: &mut EM,
    ) -> Result<(), libafl::Error> {
        self.last_corpus_count = state.corpus().count();
        let observers = executor.observers();
        let edge_observer = observers
            .get(&self.edge_observer_handle)
            .ok_or_else(|| libafl::Error::key_not_found("Cannot find edge observer"))?
            .as_ref();
        let coverage = edge_observer.count_bytes();
        let total = edge_observer.usable_count();
        let cov_precent = (coverage as f64 / total as f64) * 100.0;
        manager.log(
            state,
            LogSeverity::Info,
            format!("Coverage: {coverage} of {total} covered ({cov_precent:.2}%)"),
        )
    }
}

#[derive(Debug)]
pub struct CleanupWorkspaceDirs<S> {
    last_cleanup: u64,
    _state: PhantomData<S>,
}

impl<S> CleanupWorkspaceDirs<S> {
    pub fn new() -> Self {
        Self {
            last_cleanup: 0,
            _state: PhantomData,
        }
    }
}

impl<S> UsesState for CleanupWorkspaceDirs<S>
where
    S: State + UsesInput,
{
    type State = S;
}

impl<E, M, Z, S> Stage<E, M, Z> for CleanupWorkspaceDirs<S>
where
    S: State + UsesInput + HasExecutions,
    E: UsesState<State = S>,
    M: EventFirer + UsesState<State = S>,
    Z: UsesState<State = S>,
{
    fn should_restart(&mut self, _state: &mut Self::State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut Self::State) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut Self::State,
        manager: &mut M,
    ) -> Result<(), libafl::Error> {
        let executions = *state.executions();
        const THRESHOLD: u64 = 1000;
        if executions - self.last_cleanup > THRESHOLD {
            let cleanup_range = self.last_cleanup..executions;
            manager.log(
                state,
                LogSeverity::Info,
                format!(
                    "Cleaning up workspace directories from {} to {}",
                    cleanup_range.start, cleanup_range.end
                ),
            )?;
            thread::spawn(move || {
                for exec_num in cleanup_range {
                    let workspace_dir = temp_dir().join(format!("lsp-fuzz-workspace_{exec_num}"));
                    std::fs::remove_dir_all(workspace_dir).expect("The dir should exist");
                }
            });
            self.last_cleanup = executions;
        }
        Ok(())
    }
}
