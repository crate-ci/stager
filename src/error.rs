use std::fmt;
use std::iter;

use failure;

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
