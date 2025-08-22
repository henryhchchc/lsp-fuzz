use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use derive_new::new as New;
use libafl::{HasMetadata, observers::Observer};
use libafl_bolts::Named;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, New)]
pub struct WorkspaceObserver {
    temp_dir: PathBuf,
}

impl Named for WorkspaceObserver {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("WorkspaceObserver");
        &NAME
    }
}

pub trait HasWorkspace {
    fn workspace_hash(&self) -> u64;
    fn setup_workspace(&self, workspace_root: &Path) -> Result<(), std::io::Error>;
}

impl<Input, State> Observer<Input, State> for WorkspaceObserver
where
    State: HasMetadata,
    Input: HasWorkspace,
{
    fn pre_exec(&mut self, _state: &mut State, input: &Input) -> Result<(), libafl::Error> {
        let input_hash = input.workspace_hash();
        let workspace_dir = self
            .temp_dir
            .join(format!("lsp-fuzz-workspace_{input_hash}"));

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
        let input_hash = input.workspace_hash();
        let workspace_dir = self
            .temp_dir
            .join(format!("lsp-fuzz-workspace_{input_hash}"));

        std::fs::remove_dir_all(workspace_dir)?;

        Ok(())
    }
}
