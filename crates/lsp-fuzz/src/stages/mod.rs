use std::{env::temp_dir, marker::PhantomData, thread};

use libafl::{
    events::{EventFirer, LogSeverity},
    inputs::UsesInput,
    stages::Stage,
    state::{HasExecutions, State, UsesState},
};

#[derive(Debug)]
pub struct CleanupWorkspaceDirs<S> {
    last_cleanup: u64,
    _state: PhantomData<S>,
}

impl<S> CleanupWorkspaceDirs<S> {
    pub fn new() -> Self {
        Self::default()
    }

    const MIN_CLEANUP_DIRS: u64 = 1000;
}

impl<S> Default for CleanupWorkspaceDirs<S> {
    fn default() -> Self {
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
    fn should_restart(&mut self, state: &mut Self::State) -> Result<bool, libafl::Error> {
        Ok(*state.executions() - self.last_cleanup >= Self::MIN_CLEANUP_DIRS)
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
        Ok(())
    }
}
