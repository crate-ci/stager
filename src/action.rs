//! Filesystem operations to stage files.

use std::fmt;
use std::fs;
use std::path;

use failure;

// `Display` is required for dry-runs / previews.
/// Operation for setting up staged directory tree.
pub trait Action: fmt::Display + fmt::Debug {
    /// Execute the current action, writing to the stage.
    fn perform(&self) -> Result<(), failure::Error>;
}

/// Specifies a staged directory to be created.
#[derive(Clone, Debug)]
pub struct CreateDirectory {
    staged: path::PathBuf,
}

impl CreateDirectory {
    /// Specifies a staged directory to be created.
    ///
    /// - `staged`: full path to future directory.
    pub fn new<P>(staged: P) -> Self
    where
        P: Into<path::PathBuf>,
    {
        Self {
            staged: staged.into(),
        }
    }
}

impl fmt::Display for CreateDirectory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "mkdir {:?}", self.staged)
    }
}

impl Action for CreateDirectory {
    fn perform(&self) -> Result<(), failure::Error> {
        fs::create_dir_all(&self.staged)?;

        Ok(())
    }
}

/// Specifies a file to be staged into the target directory.
#[derive(Clone, Debug)]
pub struct CopyFile {
    staged: path::PathBuf,
    source: path::PathBuf,
}

impl CopyFile {
    /// Specifies a file to be staged into the target directory.
    ///
    /// - `staged`: full path to future file.
    /// - `source`: full path to file being written to `staged`.
    pub fn new<D, S>(staged: D, source: S) -> Self
    where
        D: Into<path::PathBuf>,
        S: Into<path::PathBuf>,
    {
        Self {
            staged: staged.into(),
            source: source.into(),
        }
    }
}

impl fmt::Display for CopyFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "cp {:?} {:?}", self.source, self.staged)
    }
}

impl Action for CopyFile {
    fn perform(&self) -> Result<(), failure::Error> {
        if let Some(parent) = self.staged.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&self.source, &self.staged)?;

        Ok(())
    }
}

/// Specifies a symbolic link file to be staged into the target directory.
#[derive(Clone, Debug)]
pub struct Symlink {
    staged: path::PathBuf,
    target: path::PathBuf,
}

impl Symlink {
    /// Specifies a symbolic link file to be staged into the target directory.
    ///
    /// - `staged`: full path for future symlink.
    /// - `target`: path that symlink will point to.
    pub fn new<S, T>(staged: S, target: T) -> Self
    where
        S: Into<path::PathBuf>,
        T: Into<path::PathBuf>,
    {
        Self {
            staged: staged.into(),
            target: target.into(),
        }
    }
}

impl fmt::Display for Symlink {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ln -s {:?} {:?}", self.target, self.staged)
    }
}

impl Action for Symlink {
    fn perform(&self) -> Result<(), failure::Error> {
        if let Some(parent) = self.staged.parent() {
            fs::create_dir_all(parent)?;
        }
        #[allow(deprecated)]
        fs::soft_link(&self.staged, &self.target)?;

        Ok(())
    }
}
