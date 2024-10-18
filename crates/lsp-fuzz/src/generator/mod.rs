use std::collections::HashMap;

use libafl::{generators::Generator, inputs::BytesInput, state::HasRand};

use crate::{
    inputs::{
        file_system::FileSystemEntryInput, LspInput, LspRequestSequence, SourceDirectoryInput,
    },
    utf8::Utf8Input,
};

#[derive(Debug)]
pub struct LspInpuGenerator;

impl<S> Generator<LspInput, S> for LspInpuGenerator
where
    S: HasRand,
{
    fn generate(&mut self, _state: &mut S) -> Result<LspInput, libafl::Error> {
        Ok(LspInput {
            requests: LspRequestSequence { requests: vec![] },
            source_directory: SourceDirectoryInput(HashMap::from([(
                Utf8Input::new("main.c".to_owned()),
                FileSystemEntryInput::File(BytesInput::new(b"int main() { return 0; }".to_vec())),
            )])),
        })
    }
}
