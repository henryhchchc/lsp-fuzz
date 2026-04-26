use crate::{lsp::GeneratorsConfig, lsp_input::LspInput, macros::append_randoms};

use super::AppendMessage;

append_randoms! {
    pub fn append_hierarchy_messages(config: &GeneratorsConfig) -> AppendHierarchyMessageMutations {
        request::CallHierarchyIncomingCalls,
        request::CallHierarchyOutgoingCalls,
        request::CallHierarchyPrepare,
        request::TypeHierarchyPrepare,
        request::TypeHierarchySubtypes,
        request::TypeHierarchySupertypes,
    }
}
