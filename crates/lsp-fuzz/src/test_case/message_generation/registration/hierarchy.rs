use crate::macros::append_randoms;

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
