use std::hash::{Hash, Hasher};

use libafl::{
    corpus::CorpusId,
    inputs::{HasMutatorBytes, Input},
};
use libafl_bolts::HasLen;
use serde::{Deserialize, Serialize};

pub mod lsp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspInput {
    bytes: Vec<u8>,
}
impl LspInput {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl Input for LspInput {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        let mut hasher = std::hash::DefaultHasher::new();
        self.bytes.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

impl HasLen for LspInput {
    fn len(&self) -> usize {
        self.bytes.len()
    }
}

impl HasMutatorBytes for LspInput {
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    fn resize(&mut self, new_len: usize, value: u8) {
        self.bytes.resize(new_len, value)
    }

    fn extend<'a, I: IntoIterator<Item = &'a u8>>(&mut self, iter: I) {
        self.bytes.extend(iter)
    }

    fn splice<R, I>(&mut self, range: R, replace_with: I) -> std::vec::Splice<'_, I::IntoIter>
    where
        R: std::ops::RangeBounds<usize>,
        I: IntoIterator<Item = u8>,
    {
        self.bytes.splice(range, replace_with)
    }

    fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, u8>
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.bytes.drain(range)
    }
}
