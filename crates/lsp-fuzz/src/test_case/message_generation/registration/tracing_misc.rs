use crate::macros::append_randoms;

append_randoms! {
    pub fn append_tracing_misc_messages(config: &GeneratorsConfig) -> AppendTracingMiscMessageMutations {
        notification::LogTrace,
        notification::SetTrace,
    }
}
