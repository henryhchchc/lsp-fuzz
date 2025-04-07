use std::{borrow::Cow, char, collections::HashSet, num::NonZero, ops::DerefMut};

use libafl::{
    inputs::Input,
    mutators::{MutationResult, Mutator},
    state::{HasMaxSize, HasRand},
};
use libafl_bolts::{Named, rands::Rand};

#[derive(Debug)]
pub struct CharInsertMutator;

impl Named for CharInsertMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("CharInsertMutator");
        &NAME
    }
}

const MAX_INSERT_SIZE: usize = 16;

impl<I, State> Mutator<I, State> for CharInsertMutator
where
    I: Input + DerefMut<Target = String>,
    State: HasRand + HasMaxSize,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        let max_size = state.max_size();
        let rand = state.rand_mut();

        let len = input.len();
        if len == 0 || len >= max_size {
            return Ok(MutationResult::Skipped);
        }

        let mut insertion_chars = 1 + rand.below(NonZero::new(MAX_INSERT_SIZE).unwrap());
        let (index, val) = rand
            .choose(input.as_str().char_indices())
            .expect("We checked that the input is not empty");

        if len + (insertion_chars * val.len_utf8()) > max_size {
            if max_size - len > val.len_utf8() {
                insertion_chars = (max_size - len) / val.len_utf8();
            } else {
                return Ok(MutationResult::Skipped);
            }
        }

        let insertion: String = (0..insertion_chars).map(|_| val).collect();
        input.insert_str(index, &insertion);

        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug)]
pub struct CharDeleteMutator;

impl Named for CharDeleteMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("CharDeleteMutator");
        &NAME
    }
}

impl<I, State> Mutator<I, State> for CharDeleteMutator
where
    I: Input + DerefMut<Target = String>,
    State: HasRand + HasMaxSize,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        let len = input.len();
        if len == 0 {
            return Ok(MutationResult::Skipped);
        }

        let rand = state.rand_mut();
        let (idx, _) = rand
            .choose(input.char_indices())
            .expect("We have checked that the string is not empty.");
        input.remove(idx);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug)]
pub struct CharReplaceMutator;

impl Named for CharReplaceMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("CharReplaceMutator");
        &NAME
    }
}

impl<I, State> Mutator<I, State> for CharReplaceMutator
where
    I: Input + DerefMut<Target = String>,
    State: HasRand + HasMaxSize,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        let len = input.len();
        if len == 0 {
            return Ok(MutationResult::Skipped);
        }

        let rand = state.rand_mut();
        let (idx, picked) = rand
            .choose(input.char_indices())
            .expect("We have checked that the string is not empty.");
        let new_char = rand
            .choose(input.chars())
            .expect("Random choice from non-empty set should not fail.");
        input.replace_range(idx..idx + picked.len_utf8(), &new_char.to_string());
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug)]
pub struct StringTruncationMutator;

impl Named for StringTruncationMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("StringTruncationMutator");
        &NAME
    }
}

impl<I, State> Mutator<I, State> for StringTruncationMutator
where
    I: Input + DerefMut<Target = String>,
    State: HasRand + HasMaxSize,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        let len = input.len();
        if len == 0 {
            return Ok(MutationResult::Skipped);
        }

        let rand = state.rand_mut();
        let Some((truncate_len, _)) = rand.choose(input.char_indices().skip(1)) else {
            return Ok(MutationResult::Skipped);
        };
        input.truncate(truncate_len);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug, Default)]
pub struct CharShiftMutator {
    /// The set of characters that should not be produced by the mutator.
    pub blacklisted_chars: HashSet<char>,
}

impl CharShiftMutator {
    /// Create a new `CharShiftMutator` with an empty blacklist.
    pub fn new() -> Self {
        Self {
            blacklisted_chars: Default::default(),
        }
    }

    /// Create a new `CharShiftMutator` with a custom blacklist.
    pub fn with_blacklisted_chars(blacklisted_chars: HashSet<char>) -> Self {
        Self { blacklisted_chars }
    }
}

impl Named for CharShiftMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("CharShiftMutator");
        &NAME
    }
}

impl<I, State> Mutator<I, State> for CharShiftMutator
where
    I: Input + DerefMut<Target = String>,
    State: HasRand + HasMaxSize,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<MutationResult, libafl::Error> {
        let len = input.len();
        if len == 0 {
            return Ok(MutationResult::Skipped);
        }

        let rand = state.rand_mut();
        let (idx, picked) = rand
            .choose(input.char_indices())
            .expect("We have checked that the string is not empty.");

        let shift_amount = rand.below_or_zero(32);
        let new_char = if rand.coinflip(0.5) {
            (picked..char::MAX).nth(shift_amount)
        } else {
            (char::MIN..picked).rev().nth(shift_amount)
        };

        if let Some(new_char) = new_char {
            if self.blacklisted_chars.contains(&new_char) {
                return Ok(MutationResult::Skipped);
            }
            input.replace_range(idx..idx + picked.len_utf8(), &new_char.to_string());
            Ok(MutationResult::Mutated)
        } else {
            Ok(MutationResult::Skipped)
        }
    }
}
