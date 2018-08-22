//! Composable file format for staging files.
//!
//! `stager::de::MapStage` is the recommended top-level staging configuration to include in a
//! packaging configuration struct.  If you need additional sources, you might want to consider
//! replacing `MapStage` and `Source`, reusing the rest.
//!
//! `Template` fields are rendered using the [liquid][liquid] template engine. No filters or tags
//! are available at this time.
//!
//! [liquid]: https://shopify.github.io/liquid/
//!
//! ## Basic Example
//!
//! ```rust
//! use std::path;
//! use stager::de;
//! use stager::de::ActionRender;
//!
//! // #[derive(Serialize, Deserialize)]
//! #[derive(Default)]
//! struct Config {
//!     stage: de::MapStage,
//! }
//! // ...
//! let engine = de::TemplateEngine::new(Default::default()).unwrap();
//! let config = Config::default();  // Dummy data
//! let stage = config.stage.format(&engine);
//! ```

use std::collections::BTreeMap;
use std::path;

use failure;

use builder;
use error;

pub use template::*;

/// Translate user-facing configuration to the staging APIs.
pub trait ActionRender {
    /// Format the serialized data into an `ActionBuilder`.
    fn format(
        &self,
        engine: &TemplateEngine,
    ) -> Result<Box<builder::ActionBuilder>, failure::Error>;
}

/// For each stage target, a list of sources to populate it with.
///
/// The target is an absolute path, treating the stage as the root.  The target supports template
/// formatting.
pub type MapStage = CustomMapStage<Source>;

/// For each stage target, a list of sources to populate it with.
///
/// The target is an absolute path, treating the stage as the root.  The target supports template
/// formatting.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CustomMapStage<R: ActionRender>(BTreeMap<Template, Vec<R>>);

impl<R: ActionRender> CustomMapStage<R> {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::Stage, failure::Error> {
        let iter = self.0.iter().map(|(target, sources)| {
            let target = abs_to_rel(&target.format(engine)?)?;
            let sources: &Vec<R> = sources;
            let mut errors = error::Errors::new();
            let sources = {
                let sources = sources.into_iter().map(|s| s.format(engine));
                let sources = error::ErrorPartition::new(sources, &mut errors);
                let sources: Vec<_> = sources.collect();
                sources
            };
            errors.ok((target, sources))
        });
        let mut errors = error::Errors::new();
        let staging = {
            let iter = error::ErrorPartition::new(iter, &mut errors);
            let staging: builder::Stage = iter.collect();
            staging
        };

        errors.ok(staging)
    }
}

impl<R: ActionRender> ActionRender for CustomMapStage<R> {
    fn format(
        &self,
        engine: &TemplateEngine,
    ) -> Result<Box<builder::ActionBuilder>, failure::Error> {
        self.format(engine).map(|a| {
            let a: Box<builder::ActionBuilder> = Box::new(a);
            a
        })
    }
}

impl<R: ActionRender> Default for CustomMapStage<R> {
    fn default() -> Self {
        Self {
            0: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
/// Content to stage.
pub enum Source {
    /// Specifies a file to be staged into the target directory.
    SourceFile(SourceFile),
    /// Specifies a collection of files to be staged into the target directory.
    SourceFiles(SourceFiles),
    /// Specifies a symbolic link file to be staged into the target directory.
    Symlink(Symlink),
    #[doc(hidden)]
    __Nonexhaustive,
}

impl ActionRender for Source {
    fn format(
        &self,
        engine: &TemplateEngine,
    ) -> Result<Box<builder::ActionBuilder>, failure::Error> {
        let value: Box<builder::ActionBuilder> = match *self {
            Source::SourceFile(ref b) => ActionRender::format(b, engine)?,
            Source::SourceFiles(ref b) => ActionRender::format(b, engine)?,
            Source::Symlink(ref b) => ActionRender::format(b, engine)?,
            Source::__Nonexhaustive => unreachable!("This is a non-public case"),
        };
        Ok(value)
    }
}

/// Specifies a file to be staged into the target directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    ///  Specifies the full path of the file to be copied into the target directory
    pub path: Template,
    /// Specifies the name the target file should be renamed as when copying from the source file.
    /// Default is the filename of the source file.
    #[serde(default)]
    pub rename: Option<Template>,
    /// Specifies symbolic links to `rename` in the same target directory.
    #[serde(default)]
    pub symlink: Option<OneOrMany<Template>>,
    #[serde(skip)]
    non_exhaustive: (),
}

impl SourceFile {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::SourceFile, failure::Error> {
        let path = path::PathBuf::from(self.path.format(engine)?);
        let symlink = self.symlink
            .as_ref()
            .map(|a| a.format(engine))
            .map_or(Ok(None), |r| r.map(Some))?
            .unwrap_or_default();
        let value = builder::SourceFile::new(path)
            .rename(self.rename
                .as_ref()
                .map(|t| t.format(engine))
                .map_or(Ok(None), |r| r.map(Some))?)
            .push_symlinks(symlink.into_iter());
        Ok(value)
    }
}

impl ActionRender for SourceFile {
    fn format(
        &self,
        engine: &TemplateEngine,
    ) -> Result<Box<builder::ActionBuilder>, failure::Error> {
        self.format(engine).map(|a| {
            let a: Box<builder::ActionBuilder> = Box::new(a);
            a
        })
    }
}

/// Specifies a collection of files to be staged into the target directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFiles {
    ///  Specifies the root path that `patterns` will be run on to identify files to be copied into
    ///  the target directory.
    pub path: Template,
    /// Specifies the pattern for executing the recursive/multifile match.
    pub pattern: OneOrMany<Template>,
    /// When true, symbolic links are followed as if they were normal directories and files.
    /// If a symbolic link is broken or is involved in a loop, an error is yielded.
    #[serde(default)]
    pub follow_links: bool,
    /// Toggles whether no results for the pattern constitutes an error.
    ///
    /// Generally, the default of `false` is best because it makes mistakes more obvious.  An
    /// example of when no results are acceptable is a default staging configuration that
    /// implements a lot of default "good enough" policy.
    #[serde(default)]
    pub allow_empty: bool,
    #[serde(skip)]
    non_exhaustive: (),
}

impl SourceFiles {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::SourceFiles, failure::Error> {
        let path = path::PathBuf::from(self.path.format(engine)?);
        let pattern = self.pattern.format(engine)?;
        let value = builder::SourceFiles::new(path)
            .push_patterns(pattern.into_iter())
            .follow_links(self.follow_links)
            .allow_empty(self.allow_empty);
        Ok(value)
    }
}

impl ActionRender for SourceFiles {
    fn format(
        &self,
        engine: &TemplateEngine,
    ) -> Result<Box<builder::ActionBuilder>, failure::Error> {
        self.format(engine).map(|a| {
            let a: Box<builder::ActionBuilder> = Box::new(a);
            a
        })
    }
}

/// Specifies a symbolic link file to be staged into the target directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symlink {
    /// The literal path for the target to point to.
    pub target: Template,
    /// Specifies the name the symlink should be given.
    /// Default is the filename of the `target`.
    #[serde(default)]
    pub rename: Option<Template>,
    #[serde(skip)]
    non_exhaustive: (),
}

impl Symlink {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::Symlink, failure::Error> {
        let target = path::PathBuf::from(self.target.format(engine)?);
        let value = builder::Symlink::new(target).rename(self.rename
            .as_ref()
            .map(|t| t.format(engine))
            .map_or(Ok(None), |r| r.map(Some))?);
        Ok(value)
    }
}

impl ActionRender for Symlink {
    fn format(
        &self,
        engine: &TemplateEngine,
    ) -> Result<Box<builder::ActionBuilder>, failure::Error> {
        self.format(engine).map(|a| {
            let a: Box<builder::ActionBuilder> = Box::new(a);
            a
        })
    }
}

fn abs_to_rel(abs: &str) -> Result<path::PathBuf, error::StagingError> {
    if !abs.starts_with('/') {
        return Err(error::ErrorKind::InvalidConfiguration
            .error()
            .set_context(format!("Path is not absolute (within the state): {}", abs)));
    }

    let rel = abs.trim_left_matches('/');
    let mut path = path::PathBuf::new();
    for part in rel.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if part == ".." {
            if !path.pop() {
                return Err(error::ErrorKind::InvalidConfiguration
                    .error()
                    .set_context(format!("Path is outside of staging root: {:?}", abs)));
            }
        } else {
            path.push(part);
        }
    }
    Ok(path)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn abs_to_rel_errors_on_rel() {
        assert!(abs_to_rel("./hello/world").is_err());
        assert!(abs_to_rel("hello/world").is_err());
    }

    #[test]
    fn abs_to_rel_reformats() {
        assert_eq!(
            abs_to_rel("/hello/world").unwrap(),
            path::PathBuf::from("hello/world")
        );
    }

    #[test]
    fn abs_to_rel_cleans_nop() {
        assert_eq!(
            abs_to_rel("/hello//world").unwrap(),
            path::PathBuf::from("hello/world")
        );
        assert_eq!(
            abs_to_rel("/hello/./world").unwrap(),
            path::PathBuf::from("hello/world")
        );
    }

    #[test]
    fn abs_to_rel_cleans_up_root() {
        assert_eq!(
            abs_to_rel("/hello/../goodbye/world").unwrap(),
            path::PathBuf::from("goodbye/world")
        );
    }

    #[test]
    fn abs_to_rel_cleans_repeated_ups() {
        assert_eq!(
            abs_to_rel("/hello/world/../../foo/bar").unwrap(),
            path::PathBuf::from("foo/bar")
        );
    }

    #[test]
    fn abs_to_rel_cleans_up_leaf() {
        assert_eq!(
            abs_to_rel("/hello/world/foo/bar/../..").unwrap(),
            path::PathBuf::from("hello/world")
        );
    }
}
