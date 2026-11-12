use std::{borrow::Cow, path::PathBuf};

use derive_more::Constructor;
use libafl::observers::Observer;
use libafl_bolts::Named;
use serde::{Deserialize, Serialize};

use crate::test_case::HasWorkspace;

#[derive(Debug, Serialize, Deserialize, Constructor)]
pub struct WorkspaceObserver {
    temp_dir: PathBuf,
}

impl Named for WorkspaceObserver {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("WorkspaceObserver");
        &NAME
    }
}

impl<Input, State> Observer<Input, State> for WorkspaceObserver
where
    Input: HasWorkspace,
{
    fn pre_exec(&mut self, _state: &mut State, input: &Input) -> Result<(), libafl::Error> {
        let workspace_dir = input.workspace_dir(&self.temp_dir);

        std::fs::create_dir_all(&workspace_dir)?;
        input.setup_workspace(&workspace_dir)?;

        Ok(())
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        input: &Input,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<(), libafl::Error> {
        let workspace_dir = input.workspace_dir(&self.temp_dir);

        std::fs::remove_dir_all(workspace_dir)?;

        Ok(())
    }
}
