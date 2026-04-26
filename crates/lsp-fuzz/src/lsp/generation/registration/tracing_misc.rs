use crate::{lsp::GeneratorsConfig, lsp_input::LspInput, macros::append_randoms};

use super::AppendMessage;

append_randoms! {
    pub fn append_tracing_misc_messages(config: &GeneratorsConfig) -> AppendTracingMiscMessageMutations {
        notification::LogTrace,
        notification::SetTrace,
    }
}
