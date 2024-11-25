#![allow(dead_code, reason = "This is an utility module.")]

pub(crate) trait OptionExt<T> {
    fn get_or_try_insert_with<F, E>(&mut self, generator: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> Result<T, E>;
    fn afl_context<S: Into<String>>(self, message: S) -> Result<T, libafl::Error>;
    fn with_afl_context<F>(self, message: F) -> Result<T, libafl::Error>
    where
        F: FnOnce() -> String;
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

    fn afl_context<S: Into<String>>(self, message: S) -> Result<T, libafl::Error> {
        self.ok_or(()).afl_context(message)
    }

    fn with_afl_context<F>(self, message: F) -> Result<T, libafl::Error>
    where
        F: FnOnce() -> String,
    {
        self.ok_or(()).with_afl_context(message)
    }
}

pub(crate) trait ResultExt<T> {
    fn afl_context<S: Into<String>>(self, message: S) -> Result<T, libafl::Error>;
    fn with_afl_context<F>(self, message: F) -> Result<T, libafl::Error>
    where
        F: FnOnce() -> String;
}

impl<T, E> ResultExt<T> for Result<T, E> {
    /// Wraps the error in an [`libafl::Error::Unknown`] with the given message.
    fn afl_context<S: Into<String>>(self, message: S) -> Result<T, libafl::Error> {
        self.map_err(|_| libafl::Error::unknown(message))
    }

    fn with_afl_context<F>(self, message: F) -> Result<T, libafl::Error>
    where
        F: FnOnce() -> String,
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
