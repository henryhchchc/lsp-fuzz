use super::{LspInput, messages::LspMessageSequence, session};
use crate::{
    libafl_support::AflContext,
    text_document::{
        TextDocument,
        generation::{GrammarContextLookup, NamedNodeGenerator, RandomRuleSelectionStrategy},
    },
};
use derive_new::new as New;
use libafl::{generators::Generator, state::HasRand};
use libafl_bolts::rands::Rand;

#[derive(Debug, New)]
pub struct LspInputGenerator<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl<State> Generator<LspInput, State> for LspInputGenerator<'_>
where
    State: HasRand,
{
    fn generate(&mut self, state: &mut State) -> Result<LspInput, libafl::Error> {
        let rand = state.rand_mut();
        let grammar = rand
            .choose(self.grammar_lookup.iter())
            .afl_context("The grammar lookup context is empry")?;
        let language = grammar.language();
        let ext = rand
            .choose(language.file_extensions())
            .afl_context("The language has no extensions")?;
        let document_content = loop {
            let selection_strategy = RandomRuleSelectionStrategy;
            let generator = NamedNodeGenerator::new(grammar, selection_strategy);
            let generate_node = generator.generate(grammar.start_symbol(), state);
            if let Ok(code) = generate_node {
                break code;
            }
        };
        let mut text_document = TextDocument::new(language, document_content.clone());
        text_document.update_metadata();

        let workspace = session::workspace_for_document(text_document, ext);
        Ok(LspInput {
            messages: LspMessageSequence::default(),
            workspace,
        })
    }
}
