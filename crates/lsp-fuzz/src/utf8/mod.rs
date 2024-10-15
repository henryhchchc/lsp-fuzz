use std::ops::Deref;

use libafl::{
    corpus::CorpusId,
    inputs::{HasTargetBytes, Input},
};
use libafl_bolts::{ownedref::OwnedSlice, HasLen};
use serde::{Deserialize, Serialize};
use tuple_list::{tuple_list, TupleList};

pub mod mutators;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Utf8Input {
    inner: String,
}

impl Input for Utf8Input {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        self.inner.clone()
    }
}

pub trait HasMutatorUtf8: HasLen {
    fn chars_count(&self) -> usize;
    fn as_str(&self) -> &str;
    fn as_mut_str(&mut self) -> &mut str;
    fn insert_str(&mut self, index: usize, s: &str);
    fn remove_char(&mut self, index: usize) -> char;
}

impl HasTargetBytes for Utf8Input {
    fn target_bytes(&self) -> OwnedSlice<'_, u8> {
        self.inner.as_bytes().into()
    }
}

impl Deref for Utf8Input {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for Utf8Input {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl HasLen for Utf8Input {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

pub fn utf8_mutations() -> impl TupleList {
    tuple_list![
        mutators::CharInsertMutator,
        mutators::CharDeleteMutator,
        mutators::CharReplaceMutator,
        mutators::CharShiftMutator,
        mutators::StringTruncationMutator,
    ]
}
