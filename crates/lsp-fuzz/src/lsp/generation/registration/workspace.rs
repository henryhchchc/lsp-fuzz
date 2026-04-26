use crate::{lsp::GeneratorsConfig, lsp_input::LspInput, macros::append_randoms};

use super::AppendMessage;

append_randoms! {
    pub fn append_workspace_messages(config: &GeneratorsConfig) -> AppendWorkspaceMessageMutations {
        request::ExecuteCommand,
        request::WorkspaceSymbolRequest,
        request::WorkspaceSymbolResolve,
    }
}
