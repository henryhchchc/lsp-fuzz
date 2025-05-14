#![allow(dead_code, reason = "This is an utility module.")]

use std::{borrow::Cow, fmt::Display, num::NonZero};

use libafl::mutators::MutatorsTuple;
use libafl_bolts::{HasLen, rands::Rand, tuples::NamedTuple};

pub(crate) trait OptionExt<T> {
    fn get_or_try_insert_with<F, E>(&mut self, generator: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> Result<T, E>;
}

impl<T> OptionExt<T> for Option<T> {
    fn get_or_try_insert_with<F, E>(&mut self, generator: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if let Some(value) = self {
            Ok(value)
        } else {
            let value = generator()?;
            *self = Some(value);
            // SAFETY: We just inserted a value, so it's safe to unwrap.
            let value = unsafe { self.as_mut().unwrap_unchecked() };
            Ok(value)
        }
    }
}

impl<T> AflContext<T> for Option<T> {
    fn afl_context<M: Into<String>>(self, message: M) -> Result<T, libafl::Error> {
        self.ok_or("Unwrapping a None").afl_context(message)
    }

    fn with_afl_context<F, M>(self, message: F) -> Result<T, libafl::Error>
    where
        F: FnOnce() -> M,
        M: Into<String>,
    {
        self.ok_or("Unwrapping a None").with_afl_context(message)
    }
}

pub(crate) trait AflContext<T> {
    fn afl_context<S: Into<String>>(self, message: S) -> Result<T, libafl::Error>;
    fn with_afl_context<F, M>(self, message: F) -> Result<T, libafl::Error>
    where
        F: FnOnce() -> M,
        M: Into<String>;
}

impl<T, E: Display> AflContext<T> for Result<T, E> {
    /// Wraps the error in an [`libafl::Error::Unknown`] with the given message.
    fn afl_context<M: Into<String>>(self, message: M) -> Result<T, libafl::Error> {
        self.map_err(|e| libafl::Error::unknown(format!("{}: {}", message.into(), e)))
    }

    fn with_afl_context<F, M>(self, message: F) -> Result<T, libafl::Error>
    where
        F: FnOnce() -> M,
        M: Into<String>,
    {
        self.map_err(|_| libafl::Error::unknown(message()))
    }
}

pub trait MapInner<T, U> {
    type MapResult;
    fn map_inner<F>(self, f: F) -> Self::MapResult
    where
        F: FnOnce(T) -> U;
}

impl<T, U, E> MapInner<T, U> for Result<Option<T>, E> {
    type MapResult = Result<Option<U>, E>;

    fn map_inner<F>(self, f: F) -> Self::MapResult
    where
        F: FnOnce(T) -> U,
    {
        self.map(|inner| inner.map(f))
    }
}

pub(crate) trait RandExt {
    fn weighted_choose<I, T>(&mut self, weighted_choices: I) -> Option<T>
    where
        I: IntoIterator<Item = (T, usize)>;
}

impl<R> RandExt for R
where
    R: Rand,
{
    fn weighted_choose<I, T>(&mut self, weighted_choices: I) -> Option<T>
    where
        I: IntoIterator<Item = (T, usize)>,
    {
        // Weighted selection
        let (range_lookup, max) = weighted_choices.into_iter().fold(
            (Vec::with_capacity(0), 0),
            |(mut map, start), (item, weight)| {
                let end = start + weight;
                map.push((start..end, item));
                (map, end)
            },
        );
        let chosen_point = self.below(NonZero::new(max)?);
        range_lookup
            .into_iter()
            .find_map(|(range, item)| range.contains(&chosen_point).then_some(item))
    }
}

#[derive(Debug)]
pub enum EitherTuple<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> HasLen for EitherTuple<L, R>
where
    L: HasLen,
    R: HasLen,
{
    fn len(&self) -> usize {
        match self {
            EitherTuple::Left(l) => l.len(),
            EitherTuple::Right(r) => r.len(),
        }
    }
}

impl<L, R> NamedTuple for EitherTuple<L, R>
where
    L: NamedTuple,
    R: NamedTuple,
{
    fn name(&self, index: usize) -> Option<&Cow<'static, str>> {
        match self {
            EitherTuple::Left(l) => l.name(index),
            EitherTuple::Right(r) => r.name(index),
        }
    }

    fn names(&self) -> Vec<Cow<'static, str>> {
        match self {
            EitherTuple::Left(l) => l.names(),
            EitherTuple::Right(r) => r.names(),
        }
    }
}

impl<L, R, I, State> MutatorsTuple<I, State> for EitherTuple<L, R>
where
    L: MutatorsTuple<I, State>,
    R: MutatorsTuple<I, State>,
{
    fn mutate_all(
        &mut self,
        state: &mut State,
        input: &mut I,
    ) -> Result<libafl::mutators::MutationResult, libafl::Error> {
        match self {
            EitherTuple::Left(l) => l.mutate_all(state, input),
            EitherTuple::Right(r) => r.mutate_all(state, input),
        }
    }

    fn post_exec_all(
        &mut self,
        state: &mut State,
        new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        match self {
            EitherTuple::Left(l) => l.post_exec_all(state, new_corpus_id),
            EitherTuple::Right(r) => r.post_exec_all(state, new_corpus_id),
        }
    }

    fn get_and_mutate(
        &mut self,
        index: libafl::mutators::MutationId,
        state: &mut State,
        input: &mut I,
    ) -> Result<libafl::mutators::MutationResult, libafl::Error> {
        match self {
            EitherTuple::Left(l) => l.get_and_mutate(index, state, input),
            EitherTuple::Right(r) => r.get_and_mutate(index, state, input),
        }
    }

    fn get_and_post_exec(
        &mut self,
        index: usize,
        state: &mut State,
        corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        match self {
            EitherTuple::Left(l) => l.get_and_post_exec(index, state, corpus_id),
            EitherTuple::Right(r) => r.get_and_post_exec(index, state, corpus_id),
        }
    }
}

pub fn generate_random_string<R: Rand>(rand: &mut R, max_length: usize) -> String {
    let length = if max_length > 0 {
        rand.below_or_zero(max_length) + 1 // Ensure we generate at least one character
    } else {
        0
    };

    let mut result = String::with_capacity(length);
    for _ in 0..length {
        // Generate a random ASCII character from the printable range (32 to 126)
        let char_code = rand.between(32, 126) as u8;
        result.push(char_code as char);
    }

    result
}
