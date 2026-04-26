#[allow(
    clippy::wildcard_imports,
    reason = "LSP parameter types are dense here"
)]
use lsp_types::*;
use tuple_list::TupleList;

use crate::lsp_input::LspInput;

compose! {
    WorkspaceSymbolParams {
        query: String,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

impl crate::lsp::Compose for ExecuteCommandParams {
    type Components = tuple_list::tuple_list_type![Command, WorkDoneProgressParams];

    fn compose(components: Self::Components) -> Self {
        let (command, work_done_progress_params) = components.into_tuple();
        Self {
            command: command.command,
            arguments: command.arguments.unwrap_or_default(),
            work_done_progress_params,
        }
    }
}

impl crate::lsp::Compose for PreviousResultId {
    type Components = tuple_list::tuple_list_type![String];

    #[inline]
    fn compose(components: Self::Components) -> Self {
        let (value,) = components.into_tuple();
        Self {
            uri: LspInput::root_uri(),
            value,
        }
    }
}
