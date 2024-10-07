#![warn(missing_debug_implementations, rust_2018_idioms)]

use std::hash::{Hash, Hasher};

use libafl::{inputs::Input, prelude::CorpusId};
use serde::{Deserialize, Serialize};

pub mod execution;
pub mod generator;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspInput {
    bytes: Vec<u8>,
}

impl Input for LspInput {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        let mut hasher = std::hash::DefaultHasher::new();
        self.bytes.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
