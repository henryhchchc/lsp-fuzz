use std::ops::{Deref, DerefMut, Index};

use derive_more::{
    Debug,
    derive::{From, Into},
};
use indexmap::IndexSet;
use libafl::{
    corpus::CorpusId,
    inputs::{HasTargetBytes, Input},
};
use libafl_bolts::{HasLen, ownedref::OwnedSlice};
use serde::{Deserialize, Serialize};
use tuple_list::{TupleList, tuple_list};

pub mod mutators;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, From, Into)]
#[serde(transparent)]
pub struct Utf8Input {
    inner: String,
}

impl Utf8Input {
    pub fn new(inner: String) -> Self {
        Self { inner }
    }
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

impl DerefMut for Utf8Input {
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
        mutators::CharShiftMutator::new(),
        mutators::StringTruncationMutator,
    ]
}

pub fn file_name_mutations() -> impl TupleList {
    tuple_list![
        mutators::CharInsertMutator,
        mutators::CharDeleteMutator,
        mutators::CharReplaceMutator,
        mutators::CharShiftMutator::with_blacklisted_chars(['/'].into()),
        mutators::StringTruncationMutator,
    ]
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, libafl_bolts::SerdeAny)]
pub struct UTF8Tokens {
    content: IndexSet<String>,
}

impl UTF8Tokens {
    pub fn new() -> Self {
        Self {
            content: IndexSet::new(),
        }
    }

    pub fn add_token(&mut self, token: String) {
        self.content.insert(token);
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.content.iter()
    }

    pub fn parse_auto_dict<B>(&mut self, payload: B)
    where
        B: IntoIterator<Item = u8>,
    {
        let mut payload = payload.into_iter();
        while let Some(size) = payload.next().map(usize::from) {
            if size > 0
                && let Ok(token) = String::from_utf8(payload.by_ref().take(size).collect())
            {
                self.content.insert(token);
            }
        }
    }
}

impl Index<usize> for UTF8Tokens {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        self.content.index(index)
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn serialize_utf8_input() {
        let input = super::Utf8Input::new("Hello, World!".to_string());
        let serialized = serde_json::to_string(&input).unwrap();
        assert_eq!(serialized, r#""Hello, World!""#);
    }
}
