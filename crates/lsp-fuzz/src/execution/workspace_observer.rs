use std::{borrow::Cow, path::PathBuf};

use derive_new::new as New;
use libafl::{HasMetadata, observers::Observer};
use libafl_bolts::Named;
use serde::{Deserialize, Serialize};

use crate::lsp_input::LspInput;

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
    State: HasMetadata,
{
    fn pre_exec(&mut self, _state: &mut State, input: &LspInput) -> Result<(), libafl::Error> {
        let input_hash = input.workspace_hash();
        let workspace_dir = self
            .temp_dir
            .join(format!("lsp-fuzz-workspace_{input_hash}"));

        std::fs::create_dir_all(&workspace_dir)?;
        input.setup_source_dir(&workspace_dir)?;

        Ok(())
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        input: &LspInput,
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
