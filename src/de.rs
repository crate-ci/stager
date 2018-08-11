//! Composable file format for staging files.
//!
//! `stager::de::Staging` is the recommended top-level staging configuration to include in a
//! packaging configuration struct.  If you need additional sources, you might want to consider
//! replacing `Staging` and `Source`, reusing the rest.
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
//!     stage: de::Staging,
//! }
//! // ...
//! let engine = de::TemplateEngine::new(Default::default()).unwrap();
//! let config = Config::default();  // Dummy data
//! let stage = config.stage.format(&engine);
//! ```

use std::collections::BTreeMap;
use std::path;

use failure;
use liquid;

use builder;
use error;

/// Translate user-facing configuration to the staging APIs.
pub trait ActionRender {
    fn format(
        &self,
        engine: &TemplateEngine,
    ) -> Result<Box<builder::ActionBuilder>, failure::Error>;
}

/// For each stage target, a list of sources to populate it with.
///
/// The target is an absolute path, treating the stage as the root.  The target supports template
/// formatting.
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct Staging(BTreeMap<Template, Vec<Source>>);

impl Staging {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::Staging, failure::Error> {
        let iter = self.0.iter().map(|(target, sources)| {
            let target = abs_to_rel(&target.format(engine)?)?;
            let sources: &Vec<Source> = sources;
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
            let staging: builder::Staging = iter.collect();
            staging
        };

        errors.ok(staging)
    }
}

impl ActionRender for Staging {
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Source {
    SourceFile(SourceFile),
    SourceFiles(SourceFiles),
    Symlink(Symlink),
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
        };
        Ok(value)
    }
}

/// Specifies a file to be staged into the target directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    ///  Specifies the full path of the file to be copied into the target directory
    path: Template,
    /// Specifies the name the target file should be renamed as when copying from the source file.
    /// Default is the filename of the source file.
    #[serde(default)]
    rename: Option<Template>,
    /// Specifies symbolic links to `rename` in the same target directory.
    #[serde(default)]
    symlink: Option<OneOrMany<Template>>,
}

impl SourceFile {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::SourceFile, failure::Error> {
        let symlink = self.symlink
            .as_ref()
            .map(|a| a.format(engine))
            .map_or(Ok(None), |r| r.map(Some))?
            .unwrap_or_default();
        let value = builder::SourceFile {
            path: path::PathBuf::from(self.path.format(engine)?),
            rename: self.rename
                .as_ref()
                .map(|t| t.format(engine))
                .map_or(Ok(None), |r| r.map(Some))?,
            symlink,
        };
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
    path: Template,
    /// Specifies the pattern for executing the recursive/multifile match.
    pattern: OneOrMany<Template>,
    #[serde(default)]
    follow_links: bool,
    /// Toggles whether no results for the pattern constitutes an error.
    ///
    /// Generally, the default of `false` is best because it makes mistakes more obvious.  An
    /// example of when no results are acceptable is a default staging configuration that
    /// implements a lot of default "good enough" policy.
    #[serde(default)]
    allow_empty: bool,
}

impl SourceFiles {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::SourceFiles, failure::Error> {
        let pattern = self.pattern.format(engine)?;
        let value = builder::SourceFiles {
            path: path::PathBuf::from(self.path.format(engine)?),
            pattern,
            follow_links: self.follow_links,
            allow_empty: self.allow_empty,
        };
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
    target: Template,
    /// Specifies the name the symlink should be given.
    /// Default is the filename of the `target`.
    rename: Template,
}

impl Symlink {
    fn format(&self, engine: &TemplateEngine) -> Result<builder::Symlink, failure::Error> {
        let value = builder::Symlink {
            target: path::PathBuf::from(self.target.format(engine)?),
            rename: self.rename.format(engine)?,
        };
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

// TODO(epage): Look into making template system pluggable
// - Leverage traits
// - Possibly get liquid to also work with serializables like Tera(?)
// But should we?  Would it be better to have consistency in syntax and functionality?
// Either way, might be better to switch to another template engine if it looks like its getting
// traction within Rust community (like whatever is used for cargo templates) and to one that will
// be 1.0 sooner.
pub struct TemplateEngine {
    pub parser: liquid::Parser,
    pub data: liquid::Object,
}

impl TemplateEngine {
    pub fn new(data: liquid::Object) -> Result<Self, failure::Error> {
        // TODO(eage): Better customize liquid
        // - Add raw block
        // - Remove irrelevant filters (like HTML ones)
        // - Add path manipulation filters
        let parser = liquid::ParserBuilder::new().liquid_filters().build();
        Ok(Self { parser, data })
    }

    pub fn render(&self, template: &str) -> Result<String, failure::Error> {
        // TODO(epage): get liquid to be compatible with failure::Fail
        let template = self.parser.parse(template)?;
        let content = template.render(&self.data)?;
        Ok(content)
    }
}

/// Translate user-facing value to a staging value.
pub trait TemplateRender {
    type Rendered;

    fn format(&self, engine: &TemplateEngine) -> Result<Self::Rendered, failure::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> TemplateRender for OneOrMany<T>
where
    T: TemplateRender,
{
    type Rendered = Vec<T::Rendered>;

    fn format(&self, engine: &TemplateEngine) -> Result<Self::Rendered, failure::Error> {
        match *self {
            OneOrMany::One(ref v) => {
                let u = v.format(engine)?;
                Ok(vec![u])
            }
            OneOrMany::Many(ref v) => {
                let u: Result<Vec<_>, _> = v.iter().map(|a| a.format(engine)).collect();
                u
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Template(String);

impl Template {
    pub fn new<S>(s: S) -> Self
    where
        S: Into<String>,
    {
        Self { 0: s.into() }
    }
}

impl TemplateRender for Template {
    type Rendered = String;

    fn format(&self, engine: &TemplateEngine) -> Result<String, failure::Error> {
        engine.render(&self.0)
    }
}

fn abs_to_rel(abs: &str) -> Result<path::PathBuf, failure::Error> {
    if !abs.starts_with('/') {
        bail!("Path is not absolute (within the state): {}", abs);
    }

    let rel = abs.trim_left_matches('/');
    let mut path = path::PathBuf::new();
    for part in rel.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if part == ".." {
            if !path.pop() {
                bail!("Path is outside of staging root: {:?}", abs);
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
