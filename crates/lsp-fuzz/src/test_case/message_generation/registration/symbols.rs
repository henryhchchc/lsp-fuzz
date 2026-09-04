use crate::macros::append_randoms;

append_randoms! {
    pub fn append_symbol_messages(config: &GeneratorsConfig) -> AppendSymbolMessageMutations {
        request::DocumentLinkRequest,
        request::DocumentLinkResolve,
        request::DocumentSymbolRequest,
        request::CodeLensRequest,
        request::CodeLensResolve,
        request::SemanticTokensFullDeltaRequest,
        request::SemanticTokensFullRequest,
        request::SemanticTokensRangeRequest,
    }
}
