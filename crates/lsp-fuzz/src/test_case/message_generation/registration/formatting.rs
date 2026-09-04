use crate::macros::append_randoms;

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
