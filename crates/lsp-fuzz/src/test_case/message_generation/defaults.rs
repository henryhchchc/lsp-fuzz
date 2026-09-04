use trait_gen::trait_gen;

use super::DefaultGenerator;
use crate::test_case::message_generation::HasGenerators;

#[trait_gen(P ->
    lsp_types::WorkDoneProgressParams,
    lsp_types::PartialResultParams,
    (),
    serde_json::Map<String, serde_json::Value>,
    serde_json::Value,
)]
impl<State: 'static> HasGenerators<State> for P {
    type Generator = DefaultGenerator<Self>;

    fn generators(
        _config: &crate::test_case::message_generation::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        [DefaultGenerator::new()]
    }
}
