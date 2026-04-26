use crate::{lsp::GeneratorsConfig, lsp_input::LspInput, macros::append_randoms};

use super::AppendMessage;

append_randoms! {
    pub fn append_diagnostic_messages(config: &GeneratorsConfig) -> AppendDiagnosticMessageMutations {
        request::CodeActionRequest,
        request::CodeActionResolveRequest,
        request::DocumentDiagnosticRequest,
        request::WorkspaceDiagnosticRefresh,
        request::WorkspaceDiagnosticRequest,
    }
}
