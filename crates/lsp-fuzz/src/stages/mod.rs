use std::{mem, path::PathBuf, thread};

use derive_new::new as New;
use libafl::{
    events::{EventFirer, LogSeverity},
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

#[derive(Debug, New)]
pub struct CleanupWorkspaceDirs {
    cleanup_dir: String,
    cleanup_threshold: u64,
}

impl<E, M, Z, S> Stage<E, M, S, Z> for CleanupWorkspaceDirs
where
    S: State + HasExecutions + HasNamedMetadata,
    M: EventFirer + UsesState<State = S>,
{
    fn should_restart(&mut self, state: &mut S) -> Result<bool, libafl::Error> {
        let LastCleanupDir(last_cleanup) =
            *state.named_metadata_or_insert_with(&self.cleanup_dir, Default::default);
        Ok(*state.executions() - last_cleanup >= self.cleanup_threshold)
    }

    fn clear_progress(&mut self, _state: &mut S) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut S,
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
