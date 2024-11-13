use std::{marker::PhantomData, path::PathBuf, thread};

use libafl::{
    events::{EventFirer, LogSeverity},
    inputs::UsesInput,
    stages::Stage,
    state::{HasExecutions, State, UsesState},
    HasNamedMetadata,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

pub mod minimize;

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, libafl_bolts::SerdeAny)]
#[repr(transparent)]
pub struct LastCleanupDir(u64);

#[derive(Debug)]
pub struct CleanupWorkspaceDirs<S> {
    cleanup_dir: String,
    cleanup_threshold: u64,
    _state: PhantomData<S>,
}

impl<S> CleanupWorkspaceDirs<S> {
    pub fn new(cleanup_dir: String, cleanup_threshold: u64) -> Self {
        Self {
            cleanup_dir,
            cleanup_threshold,
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
    S: State + UsesInput + HasExecutions + HasNamedMetadata,
    E: UsesState<State = S>,
    M: EventFirer + UsesState<State = S>,
    Z: UsesState<State = S>,
{
    fn should_restart(&mut self, state: &mut Self::State) -> Result<bool, libafl::Error> {
        let LastCleanupDir(last_cleanup) =
            *state.named_metadata_or_insert_with(&self.cleanup_dir, Default::default);
        Ok(*state.executions() - last_cleanup >= self.cleanup_threshold)
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
        let LastCleanupDir(last_cleanup) = state.named_metadata_mut(&self.cleanup_dir)?;
        let cleanup_range = *last_cleanup..executions;
        *last_cleanup = executions;
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
