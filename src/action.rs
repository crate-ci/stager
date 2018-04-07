use std::fmt;
use std::fs;
use std::path;

use failure;

/// Operation for setting up staged directory tree.
pub trait Action: fmt::Display {
    fn perform(&self) -> Result<(), failure::Error>;
}

/// Specifies a staged directory to be created.
#[derive(Clone, Debug)]
pub struct CreateDirectory {
    /// Staged file to perform op on.
    staged: path::PathBuf,
}

impl CreateDirectory {
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
    /// Staged file to perform op on.
    staged: path::PathBuf,
    ///  Specifies the full path of the file to be copied into the `staged` file.
    source: path::PathBuf,
}

impl CopyFile {
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
    /// Staged file to perform op on.
    staged: path::PathBuf,
    /// The literal path for the target to point to.
    target: path::PathBuf,
}

impl Symlink {
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

#[derive(Clone, Debug)]
pub struct Access {
    /// Staged file to perform op on.
    staged: path::PathBuf,
    /// Specifies  permissions to be applied to the `staged` file.
    op: String,
}

impl Access {
    pub fn new<P, S>(staged: P, op: S) -> Self
    where
        P: Into<path::PathBuf>,
        S: Into<String>,
    {
        Self {
            staged: staged.into(),
            op: op.into(),
        }
    }
}

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TODO(epage): Figure out what is best here")
    }
}

impl Action for Access {
    fn perform(&self) -> Result<(), failure::Error> {
        bail!("TODO(epage): Figure out what is best here");
    }
}
