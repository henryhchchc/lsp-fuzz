use super::LspInput;
use libafl::{
    HasMetadata,
    corpus::CorpusId,
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand},
};
use libafl_bolts::{Named, rands::Rand};
use std::borrow::Cow;

#[derive(Debug)]
pub struct LspInputMutator<TM, RM> {
    text_document_mutator: TM,
    requests_mutator: RM,
}

impl<TM, RM> LspInputMutator<TM, RM> {
    /// Creates a mutator for workspace documents and LSP messages.
    #[must_use]
    pub const fn new(text_document_mutator: TM, requests_mutator: RM) -> Self {
        Self {
            text_document_mutator,
            requests_mutator,
        }
    }
}

impl<TM, RM> Named for LspInputMutator<TM, RM> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("LspInputMutator");
        &NAME
    }
}

impl<TM, RM, State> Mutator<LspInput, State> for LspInputMutator<TM, RM>
where
    TM: Mutator<LspInput, State>,
    RM: Mutator<LspInput, State>,
    State: HasMetadata + HasCorpus<LspInput> + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let mut result = MutationResult::Skipped;
        if state.rand_mut().coinflip(0.5)
            && self.text_document_mutator.mutate(state, input)? == MutationResult::Mutated
        {
            result = MutationResult::Mutated;
        }
        if self.requests_mutator.mutate(state, input)? == MutationResult::Mutated {
            result = MutationResult::Mutated;
        }
        Ok(result)
    }

    fn post_exec(
        &mut self,
        state: &mut State,
        new_corpus_id: Option<CorpusId>,
    ) -> Result<(), libafl::Error> {
        self.text_document_mutator.post_exec(state, new_corpus_id)?;
        self.requests_mutator.post_exec(state, new_corpus_id)?;
        Ok(())
    }
}
