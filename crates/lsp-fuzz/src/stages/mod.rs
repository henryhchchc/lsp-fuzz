use std::{
    fmt::Debug,
    io::{self, Write},
    marker::PhantomData,
    mem,
    path::PathBuf,
    sync::mpsc::Receiver,
    thread,
    time::Duration,
};

use derive_new::new as New;
use libafl::{
    HasNamedMetadata, SerdeAny,
    corpus::Corpus,
    events::{Event, EventFirer, EventWithStats, LogSeverity},
    feedbacks::{MapFeedback, MapFeedbackMetadata},
    observers::MapObserver,
    stages::{Restartable, Stage},
    state::{HasCorpus, HasExecutions, HasStartTime},
};
use libafl_bolts::{Named, current_time, serdeany::SerdeAny};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{lsp_input::LspInput, utils::AflContext};

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, SerdeAny)]
#[repr(transparent)]
pub struct LastCleanupDir(u64);

#[derive(Debug, New)]
pub struct CleanupWorkspaceDirs {
    cleanup_dir: String,
    cleanup_threshold: u64,
}

impl<State> Restartable<State> for CleanupWorkspaceDirs
where
    State: HasExecutions + HasNamedMetadata,
{
    fn should_restart(&mut self, state: &mut State) -> Result<bool, libafl::Error> {
        let LastCleanupDir(last_cleanup) =
            *state.named_metadata_or_insert_with(&self.cleanup_dir, Default::default);
        Ok(*state.executions() - last_cleanup >= self.cleanup_threshold)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, M, Z, State> Stage<E, M, State, Z> for CleanupWorkspaceDirs
where
    State: HasExecutions + HasNamedMetadata,
    M: EventFirer<LspInput, State>,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        manager: &mut M,
    ) -> Result<(), libafl::Error> {
        let executions = *state.executions();
        let LastCleanupDir(last_cleanup) = state.named_metadata_mut(&self.cleanup_dir)?;
        let cleanup_range = mem::replace(last_cleanup, executions)..executions;
        manager.log(
            state,
            LogSeverity::Info,
            format!(
                "Cleaning up workspace directories from {} to {}",
                cleanup_range.start, cleanup_range.end
            ),
        )?;
        let workspace_path = PathBuf::from(&self.cleanup_dir);
        thread::spawn(move || {
            for exec_num in cleanup_range {
                let workspace_dir = workspace_path.join(format!("lsp-fuzz-workspace_{exec_num}"));
                std::fs::remove_dir_all(&workspace_dir).unwrap_or_else(|err| {
                    warn!(
                        dir = %workspace_dir.display(),
                        err = %err,
                        "Failed to remove workspace directory"
                    )
                });
            }
        });
        Ok(())
    }
}

#[derive(Debug, New)]
pub struct StopOnReceived<I> {
    receiver: Receiver<()>,
    _input: PhantomData<I>,
}

impl<I, State> Restartable<State> for StopOnReceived<I> {
    fn should_restart(&mut self, _state: &mut State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, M, Z, I, State> Stage<E, M, State, Z> for StopOnReceived<I>
where
    State: HasExecutions,
    M: EventFirer<I, State>,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        manager: &mut M,
    ) -> Result<(), libafl::Error> {
        if self.receiver.try_recv().is_ok() {
            let executions = state.executions();
            let event = EventWithStats::with_current_time(Event::Stop, *executions);
            manager.fire(state, event)?;
        }
        Ok(())
    }
}

#[derive(Debug, New)]
pub struct TimeoutStopStage<I> {
    timeout: Duration,
    _input: PhantomData<I>,
}

impl<I, State> Restartable<State> for TimeoutStopStage<I> {
    fn should_restart(&mut self, _state: &mut State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, M, Z, I, State> Stage<E, M, State, Z> for TimeoutStopStage<I>
where
    State: HasStartTime + HasExecutions,
    M: EventFirer<I, State>,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        manager: &mut M,
    ) -> Result<(), libafl::Error> {
        let start_time = *state.start_time();
        let current_time = libafl_bolts::current_time();
        if current_time - start_time > self.timeout {
            let executions = state.executions();
            let event = EventWithStats::with_current_time(Event::Stop, *executions);
            manager.fire(state, event)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct StatsStage<W, O, I> {
    stats_writer: W,
    coverage_feedback_name: String,
    _phantom: PhantomData<(O, I)>,
}

impl<W, O, I, State> Restartable<State> for StatsStage<W, O, I> {
    fn should_restart(&mut self, _state: &mut State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, EM, State, Z, W, I, O> Stage<E, EM, State, Z> for StatsStage<W, O, I>
where
    W: Write,
    State: HasCorpus<I> + HasExecutions + HasStartTime + HasNamedMetadata,
    O: MapObserver,
    MapFeedbackMetadata<O::Entry>: SerdeAny,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        _manager: &mut EM,
    ) -> Result<(), libafl::Error> {
        let corpus_count = state.corpus().count();
        let time = (current_time() - *state.start_time()).as_secs();
        let exec = *state.executions();

        let cov_feedback_meta = state
            .named_metadata::<MapFeedbackMetadata<O::Entry>>(&self.coverage_feedback_name)
            .afl_context("Looking up coverage metadata")?;
        let edges_found = cov_feedback_meta.num_covered_map_indexes;

        self.write_stat(corpus_count, time, exec, edges_found)
            .afl_context("Writing stat")?;
        Ok(())
    }
}

impl<W, O, I> StatsStage<W, O, I> {
    pub fn new<C, N, R>(stats_writer: W, map_feedback: &MapFeedback<C, N, O, R>) -> Self {
        Self {
            stats_writer,
            coverage_feedback_name: map_feedback.name().clone().into_owned(),
            _phantom: PhantomData,
        }
    }

    fn write_stat(
        &mut self,
        corpus_count: usize,
        time: u64,
        exec: u64,
        edges_found: usize,
    ) -> io::Result<()>
    where
        W: Write,
    {
        writeln!(
            self.stats_writer,
            "{corpus_count},{time},{exec},{edges_found}"
        )?;
        self.stats_writer.flush()?;
        Ok(())
    }
}
