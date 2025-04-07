use std::{mem, path::PathBuf, sync::mpsc::Receiver, thread};

use derive_new::new as New;
use libafl::{
    HasNamedMetadata, SerdeAny,
    events::{Event, EventFirer, EventWithStats, LogSeverity},
    stages::{Restartable, Stage},
    state::HasExecutions,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::lsp_input::LspInput;

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
pub struct StopOnReceived<State> {
    receiver: Receiver<()>,
    _phantom: std::marker::PhantomData<State>,
}

impl<State> Restartable<State> for StopOnReceived<State> {
    fn should_restart(&mut self, _state: &mut State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, M, Z, State> Stage<E, M, State, Z> for StopOnReceived<State>
where
    State: HasExecutions,
    M: EventFirer<LspInput, State>,
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
