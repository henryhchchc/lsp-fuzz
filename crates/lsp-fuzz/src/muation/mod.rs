use std::borrow::Cow;

use libafl::{
    inputs::{BytesInput, UsesInput},
    mutators::{MutationResult, Mutator},
    state::{HasCorpus, HasMaxSize, HasRand, State},
    HasMetadata,
};
use libafl_bolts::Named;

use crate::{
    inputs::{file_system::FileSystemEntryInput::File, LspInput, SourceDirectoryInput},
    utf8::Utf8Input,
};

pub mod file_system;

#[derive(Debug)]
pub struct LspInputMutator<M> {
    inner_mutator: M,
}

impl<M> LspInputMutator<M> {
    pub fn new(inner_mutator: M) -> Self {
        Self { inner_mutator }
    }
}

impl<M> Named for LspInputMutator<M> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &Cow::Borrowed("LspInputMutator")
    }
}

impl<M, S> Mutator<LspInput, S> for LspInputMutator<M>
where
    M: Mutator<BytesInput, S>,
    S: State + UsesInput<Input = LspInput> + HasMetadata + HasCorpus + HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let path = Utf8Input::new("main.c".to_owned());
        let SourceDirectoryInput(entries) = &mut input.source_directory;
        let File(file_content) = entries.get_mut(&path).expect("This is the only file.") else {
            unreachable!("This is the only file.")
        };
        self.inner_mutator.mutate(state, file_content)
    }
}
