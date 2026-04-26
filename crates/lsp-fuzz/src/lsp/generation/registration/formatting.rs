use crate::{lsp::GeneratorsConfig, lsp_input::LspInput, macros::append_randoms};

use super::AppendMessage;

append_randoms! {
    pub fn append_formatting_messages(config: &GeneratorsConfig) -> AppendFormattingMessageMutations {
        request::Formatting,
        request::OnTypeFormatting,
        request::RangeFormatting,
        request::DocumentColor,
        request::ColorPresentationRequest,
        request::FoldingRangeRequest,
    }
}
