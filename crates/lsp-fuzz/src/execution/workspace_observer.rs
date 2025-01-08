use std::{borrow::Cow, env::temp_dir};

use libafl::{observers::Observer, state::HasExecutions};
use libafl_bolts::Named;
use serde::Serialize;

use crate::lsp_input::LspInput;

#[derive(Debug, Serialize)]
pub struct WorkSpaceObserver;

impl Named for WorkSpaceObserver {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("WorkSpaceObserver");
        &NAME
    }
}

impl<S> Observer<LspInput, S> for WorkSpaceObserver
where
    S: HasExecutions,
{
    fn pre_exec(&mut self, state: &mut S, input: &LspInput) -> Result<(), libafl::Error> {
        let workspace_dir = temp_dir().join(format!("lsp-fuzz-workspace_{}", state.executions()));
        std::fs::create_dir_all(&workspace_dir)?;
        input.setup_source_dir(&workspace_dir)?;

        Ok(())
    }
}
