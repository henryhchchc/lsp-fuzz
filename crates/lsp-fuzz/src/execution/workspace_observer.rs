use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use derive_new::new as New;
use libafl::{HasMetadata, SerdeAny, observers::Observer, state::HasExecutions};
use libafl_bolts::Named;
use serde::{Deserialize, Serialize};

use crate::lsp_input::LspInput;

#[derive(Debug, Serialize, Deserialize, Default, SerdeAny)]
pub struct CurrentWorkspaceMetadata {
    pub workspace_dir: PathBuf,
}

impl CurrentWorkspaceMetadata {
    pub fn path(&self) -> &Path {
        &self.workspace_dir
    }
}

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

impl<State> Observer<LspInput, State> for WorkspaceObserver
where
    State: HasExecutions + HasMetadata,
{
    fn pre_exec(&mut self, state: &mut State, input: &LspInput) -> Result<(), libafl::Error> {
        let workspace_dir = self
            .temp_dir
            .join(format!("lsp-fuzz-workspace_{}", state.executions()));
        let workspace_metadata: &mut CurrentWorkspaceMetadata =
            state.metadata_or_insert_with(Default::default);
        workspace_metadata.workspace_dir = workspace_dir;

        std::fs::create_dir_all(workspace_metadata.path())?;
        input.setup_source_dir(workspace_metadata.path())?;

        Ok(())
    }
}
