use crate::test_case::message_generation::append_randoms;

append_randoms! {
    pub fn append_workspace_messages(config: &GeneratorsConfig) -> AppendWorkspaceMessageMutations {
        request::ExecuteCommand,
        request::WorkspaceSymbolRequest,
        request::WorkspaceSymbolResolve,
    }
}
