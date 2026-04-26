use crate::{lsp::GeneratorsConfig, lsp_input::LspInput, macros::append_randoms};

use super::AppendMessage;

append_randoms! {
    pub fn append_navigation_messages(config: &GeneratorsConfig) -> AppendNavigationMessageMutations {
        request::GotoDeclaration,
        request::GotoDefinition,
        request::GotoImplementation,
        request::GotoTypeDefinition,
        request::HoverRequest,
        request::DocumentHighlightRequest,
        request::Completion,
        request::MonikerRequest,
        request::PrepareRenameRequest,
        request::References,
        request::Rename,
        request::SelectionRangeRequest,
        request::SignatureHelpRequest,
        request::LinkedEditingRange,
        request::InlayHintRequest,
        request::InlayHintResolveRequest,
    }
}
