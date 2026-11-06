use crate::test_case::message_generation::append_randoms;

append_randoms! {
    pub fn append_diagnostic_messages(config: &GeneratorsConfig) -> AppendDiagnosticMessageMutations {
        request::CodeActionRequest,
        request::CodeActionResolveRequest,
        request::DocumentDiagnosticRequest,
        request::WorkspaceDiagnosticRefresh,
        request::WorkspaceDiagnosticRequest,
    }
}
