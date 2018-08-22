use std::error::Error;
use std::fmt;
use std::iter;

use failure;

type ErrorCause = Error + Send + Sync + 'static;

pub struct ErrorPartition<'e, I> {
    iter: I,
    errors: &'e mut Errors,
}

impl<'e, I, T> ErrorPartition<'e, I>
where
    I: Iterator<Item = Result<T, failure::Error>>,
{
    pub fn new(iter: I, errors: &'e mut Errors) -> Self {
        Self { iter, errors }
    }
}

impl<'e, I, T> Iterator for ErrorPartition<'e, I>
where
    I: Iterator<Item = Result<T, failure::Error>>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        for item in &mut self.iter {
            match item {
                Ok(item) => return Some(item),
                Err(item) => self.errors.push(item),
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'e, I> fmt::Debug for ErrorPartition<'e, I>
where
    I: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ErrorPartition")
            .field("iter", &self.iter)
            .field("errors", &self.errors)
            .finish()
    }
}

#[derive(Debug)]
pub struct Errors {
    errors: Vec<failure::Error>,
}

impl Errors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, error: failure::Error) {
        self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn ok<T>(self, value: T) -> Result<T, failure::Error> {
        if self.is_empty() {
            Ok(value)
        } else {
            Err(self.into())
        }
    }
}

impl failure::Fail for Errors {
    fn cause(&self) -> Option<&failure::Fail> {
        None
    }

    fn backtrace(&self) -> Option<&failure::Backtrace> {
        None
    }
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for error in &self.errors {
            writeln!(f, "{}", error)?;
        }
        Ok(())
    }
}

impl iter::FromIterator<failure::Error> for Errors {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = failure::Error>,
    {
        let errors = iter.into_iter().collect();
        Self { errors }
    }
}

/// For programmatically processing failures.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidConfiguration,
    HarvestingFailed,
    StagingFailed,
}

impl ErrorKind {
    pub(crate) fn error(self) -> StagingError {
        StagingError::new(self)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorKind::InvalidConfiguration => write!(f, "Error in the configuration."),
            ErrorKind::HarvestingFailed => write!(f, "Preparing to stage failed."),
            ErrorKind::StagingFailed => write!(f, "Staging failed."),
        }
    }
}

#[derive(Debug)]
pub struct StagingError {
    kind: ErrorKind,
    context: Option<String>,
    cause: Option<Box<ErrorCause>>,
}

impl StagingError {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            context: None,
            cause: None,
        }
    }

    pub(crate) fn set_context<S>(mut self, context: S) -> Self
    where
        S: Into<String>,
    {
        let context = context.into();
        self.context = Some(context);
        self
    }

    pub(crate) fn set_cause<E>(mut self, cause: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        let cause = Box::new(cause);
        self.cause = Some(cause);
        self
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl Error for StagingError {
    fn description(&self) -> &str {
        "Staging failed."
    }

    fn cause(&self) -> Option<&Error> {
        self.cause.as_ref().map(|c| {
            let c: &Error = c.as_ref();
            c
        })
    }
}

impl fmt::Display for StagingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Staging failed: {}", self.kind)?;
        if let Some(ref context) = self.context {
            writeln!(f, "{}", context)?;
        }
        if let Some(ref cause) = self.cause {
            writeln!(f, "Cause: {}", cause)?;
        }
        Ok(())
    }
}
