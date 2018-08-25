//! Staging errors.

use std::error::Error;
use std::fmt;
use std::iter;
use std::vec;

type ErrorCause = Error + Send + Sync + 'static;

pub(crate) struct ErrorPartition<'e, I> {
    iter: I,
    errors: &'e mut Errors,
}

impl<'e, I, T> ErrorPartition<'e, I>
where
    I: Iterator<Item = Result<T, StagingError>>,
{
    pub fn new(iter: I, errors: &'e mut Errors) -> Self {
        Self { iter, errors }
    }
}

impl<'e, I, T> Iterator for ErrorPartition<'e, I>
where
    I: Iterator<Item = Result<T, StagingError>>,
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

/// Aggregation of errors from a staging operation.
#[derive(Debug)]
pub struct Errors {
    errors: Vec<StagingError>,
}

impl Errors {
    pub(crate) fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub(crate) fn with_error(error: StagingError) -> Self {
        let errors = vec![error];
        Self { errors }
    }

    pub(crate) fn push(&mut self, error: StagingError) {
        self.errors.push(error);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(crate) fn ok<T>(self, value: T) -> Result<T, Errors> {
        if self.is_empty() {
            Ok(value)
        } else {
            Err(self)
        }
    }
}

impl Error for Errors {
    fn description(&self) -> &str {
        "Processing failed."
    }

    fn cause(&self) -> Option<&Error> {
        // Can't handle this until we move off of `failure`.
        None
    }
}

impl From<StagingError> for Errors {
    fn from(error: StagingError) -> Self {
        Errors::with_error(error)
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

impl iter::FromIterator<StagingError> for Errors {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = StagingError>,
    {
        let errors = iter.into_iter().collect();
        Self { errors }
    }
}

impl Extend<StagingError> for Errors {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = StagingError>,
    {
        self.errors.extend(iter)
    }
}

impl IntoIterator for Errors {
    type Item = StagingError;
    type IntoIter = ErrorsIter;

    fn into_iter(self) -> ErrorsIter {
        ErrorsIter {
            0: self.errors.into_iter(),
        }
    }
}

/// Iterate over errors from a staging operation;
#[derive(Debug)]
pub struct ErrorsIter(vec::IntoIter<StagingError>);

impl Iterator for ErrorsIter {
    type Item = StagingError;

    #[inline]
    fn next(&mut self) -> Option<StagingError> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.0.count()
    }
}

/// For programmatically processing failures.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Error in the configuration.
    InvalidConfiguration,
    /// Preparing to stage failed.
    HarvestingFailed,
    /// Staging failed.
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

/// Single staging failure.
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

    /// Programmtically process failure.
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
