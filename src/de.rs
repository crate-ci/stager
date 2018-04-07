use std::collections::BTreeMap;
use std::path;

use builder;
use failure;

pub type Staging = BTreeMap<path::PathBuf, Vec<Source>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Source {
    Directory(Directory),
    SourceFile(SourceFile),
    SourceFiles(SourceFiles),
    Symlink(Symlink),
}

impl Source {
    pub fn format(self) -> Result<Box<builder::ActionBuilder>, failure::Error> {
        let value: Box<builder::ActionBuilder> = match self {
            Source::Directory(b) => Box::new(b.format()?),
            Source::SourceFile(b) => Box::new(b.format()?),
            Source::SourceFiles(b) => Box::new(b.format()?),
            Source::Symlink(b) => Box::new(b.format()?),
        };
        Ok(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(v) => vec![v],
            OneOrMany::Many(v) => v,
        }
    }
}

/// Override the default settings for the target directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Directory {
    access: OneOrMany<Access>,
}

impl Directory {
    pub fn format(self) -> Result<builder::Directory, failure::Error> {
        let access: Result<Vec<_>, failure::Error> = self.access
            .into_vec()
            .into_iter()
            .map(|a| a.format())
            .collect();
        let value = builder::Directory { access: access? };
        Ok(value)
    }
}

/// Specifies a file to be staged into the target directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    ///  Specifies the full path of the file to be copied into the target directory
    path: String,
    /// Specifies the name the target file should be renamed as when copying from the source file.
    /// Default is the filename of the source file.
    #[serde(default)]
    rename: Option<String>,
    /// Specifies symbolic links to `rename` in the same target directory and using the same
    /// `access`.
    #[serde(default)]
    symlink: Option<OneOrMany<String>>,
    #[serde(default)] access: Option<OneOrMany<Access>>,
}

impl SourceFile {
    pub fn format(self) -> Result<builder::SourceFile, failure::Error> {
        let access: Result<Vec<_>, failure::Error> = self.access
            .map(|a| a.into_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.format())
            .collect();
        let value = builder::SourceFile {
            path: self.path,
            rename: self.rename,
            symlink: self.symlink.map(|s| s.into_vec()).unwrap_or_default(),
            access: access?,
        };
        Ok(value)
    }
}

/// Specifies a collection of files to be staged into the target directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFiles {
    ///  Specifies the root path that `patterns` will be run on to identify files to be copied into
    ///  the target directory.
    path: String,
    /// Specifies the pattern for executing the recursive/multifile match.
    pattern: OneOrMany<String>,
    #[serde(default)] follow_links: bool,
    #[serde(default)] access: Option<OneOrMany<Access>>,
}

impl SourceFiles {
    pub fn format(self) -> Result<builder::SourceFiles, failure::Error> {
        let access: Result<Vec<_>, failure::Error> = self.access
            .map(|a| a.into_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.format())
            .collect();
        let value = builder::SourceFiles {
            path: self.path,
            pattern: self.pattern.into_vec(),
            follow_links: self.follow_links,
            access: access?,
        };
        Ok(value)
    }
}

/// Specifies a symbolic link file to be staged into the target directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symlink {
    /// The literal path for the target to point to.
    target: String,
    /// Specifies the name the symlink should be given.
    /// Default is the filename of the `target`.
    rename: String,
    #[serde(default)] access: Option<OneOrMany<Access>>,
}

impl Symlink {
    pub fn format(self) -> Result<builder::Symlink, failure::Error> {
        let access: Result<Vec<_>, failure::Error> = self.access
            .map(|a| a.into_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.format())
            .collect();
        let value = builder::Symlink {
            target: self.target,
            rename: self.rename,
            access: access?,
        };
        Ok(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    /// Specifies  permissions to be applied to the file.
    op: String,
}

impl Access {
    pub fn format(self) -> Result<builder::Access, failure::Error> {
        let value = builder::Access { op: self.op };
        Ok(value)
    }
}
