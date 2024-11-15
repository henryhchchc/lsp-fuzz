use std::str::FromStr;

use lsp_types::{
    request::{Request, SemanticTokensFullRequest},
    PartialResultParams, TextDocumentIdentifier, WorkDoneProgressParams,
};

use super::LspParamsGen;

impl LspParamsGen for <SemanticTokensFullRequest as Request>::Params {
    fn generate_one<S>(_state: &mut S, _input: &crate::lsp_input::LspInput) -> Self {
        let document_uri = lsp_types::Uri::from_str("workspace://main.c").unwrap();
        let text_document = TextDocumentIdentifier { uri: document_uri };
        Self {
            text_document,
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }
}
