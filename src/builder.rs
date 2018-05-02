//! High-level requirements for staging files.

use std::collections::BTreeMap;
use std::ffi;
use std::iter;
use std::path;

use failure;
use globwalk;

use action;
use error;

pub trait ActionBuilder {
    // TODO(epage):
    // - Change to `Iterator`.
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error>;
}

/// For each stage target, a list of sources to populate it with.
///
/// The target is a path relative to the stage root.
#[derive(Default)]
pub struct Staging(BTreeMap<path::PathBuf, Vec<Box<ActionBuilder>>>);

impl ActionBuilder for Staging {
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error> {
        let staging: Result<Vec<_>, _> = self.0
            .iter()
            .map(|(target, sources)| {
                if target.is_absolute() {
                    bail!("target must be relative to the stage root: {:?}", target);
                }
                let target = target_dir.join(target);
                let mut errors = error::Errors::new();
                let sources = {
                    let sources = sources.into_iter().map(|s| s.build(&target));
                    let sources = error::ErrorPartition::new(sources, &mut errors);
                    let sources: Vec<_> = sources.collect();
                    sources
                };
                errors.ok(sources)
            })
            .collect();
        let staging = staging?;
        let staging: Vec<_> = staging
            .into_iter()
            .flat_map(|v| v.into_iter().flat_map(|v: Vec<_>| v.into_iter()))
            .collect();
        Ok(staging)
    }
}

impl iter::FromIterator<(path::PathBuf, Vec<Box<ActionBuilder>>)> for Staging {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (path::PathBuf, Vec<Box<ActionBuilder>>)>,
    {
        let staging = iter.into_iter().collect();
        Self { 0: staging }
    }
}

/// Override the default settings for the target directory.
#[derive(Clone, Debug)]
pub struct Directory {
    pub access: Vec<Access>,
}

impl ActionBuilder for Directory {
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error> {
        let create: Box<action::Action> = Box::new(action::CreateDirectory::new(target_dir));

        let mut actions = vec![create];
        actions.extend(self.access.iter().cloned().map(|a| {
            let a = action::Access::new(target_dir, a.op);
            let a: Box<action::Action> = Box::new(a);
            a
        }));

        Ok(actions)
    }
}

/// Specifies a file to be staged into the target directory.
#[derive(Clone, Debug)]
pub struct SourceFile {
    ///  Specifies the full path of the file to be copied into the target directory
    pub path: path::PathBuf,
    /// Specifies the name the target file should be renamed as when copying from the source file.
    /// Default is the filename of the source file.
    pub rename: Option<String>,
    pub access: Vec<Access>,
    /// Specifies symbolic links to `rename` in the same target directory and using the same
    /// `access`.
    pub symlink: Vec<String>,
}

impl ActionBuilder for SourceFile {
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error> {
        let path = self.path.as_path();
        if !path.is_absolute() {
            bail!("SourceFile path must be absolute: {:?}", path);
        }

        let filename = self.rename
            .as_ref()
            .map(|n| ffi::OsStr::new(n))
            .unwrap_or_else(|| path.file_name().unwrap_or_default());
        let filename = path::Path::new(filename);
        if filename.file_name() != Some(filename.as_os_str()) {
            bail!(
                "SourceFile rename must not change directories: {:?}",
                filename
            );
        }
        let copy_target = target_dir.join(filename);
        let copy: Box<action::Action> = Box::new(action::CopyFile::new(&copy_target, path));

        let mut actions = vec![copy];
        actions.extend(self.access.iter().cloned().map(|a| {
            let a = action::Access::new(target_dir, a.op);
            let a: Box<action::Action> = Box::new(a);
            a
        }));
        actions.extend(self.symlink.iter().map(|s| {
            let s = path::Path::new(s);
            // TODO(epage): Re-enable this error check
            //if s.file_name() != Some(s.as_os_str()) {
            //    bail!("SourceFile symlink must not change directories: {:?}", s);
            //}
            let sym_target = target_dir.join(s);
            let a: Box<action::Action> = Box::new(action::Symlink::new(sym_target, &copy_target));
            a
        }));
        // TODO(epage): Set symlink permissions

        Ok(actions)
    }
}

/// Specifies a collection of files to be staged into the target directory.
#[derive(Clone, Debug)]
pub struct SourceFiles {
    ///  Specifies the root path that `pattern` will be run on to identify files to be copied into
    ///  the target directory.
    pub path: path::PathBuf,
    /// Specifies the `pattern` for executing the recursive/multifile match.
    ///
    /// `pattern` uses [gitignore][gitignore] syntax.
    ///
    /// [gitignore]: https://git-scm.com/docs/gitignore#_pattern_format
    pub pattern: Vec<String>,
    pub follow_links: bool,
    /// Toggles whether no results for the pattern constitutes an error.
    ///
    /// Generally, the default of `false` is best because it makes mistakes more obvious.  An
    /// example of when no results are acceptable is a default staging configuration that
    /// implements a lot of default "good enough" policy.
    pub allow_empty: bool,
    pub access: Vec<Access>,
}

impl ActionBuilder for SourceFiles {
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error> {
        let mut actions: Vec<Box<action::Action>> = Vec::new();
        let source_root = self.path.as_path();
        if !source_root.is_absolute() {
            bail!("SourceFiles path must be absolute: {:?}", source_root);
        }
        for entry in globwalk::GlobWalker::from_patterns(source_root, &self.pattern)?
            .follow_links(self.follow_links)
        {
            let entry = entry?;
            let source_file = entry.path();
            if source_file.is_dir() {
                continue;
            }
            let rel_source = source_file.strip_prefix(source_root)?;
            let copy_target = target_dir.join(rel_source);
            let copy: Box<action::Action> =
                Box::new(action::CopyFile::new(&copy_target, source_file));
            actions.push(copy);

            actions.extend(self.access.iter().cloned().map(|a| {
                let a = action::Access::new(target_dir, a.op);
                let a: Box<action::Action> = Box::new(a);
                a
            }));
        }

        if actions.is_empty() {
            if self.allow_empty {
                info!(
                    "No files found under {:?} with patterns {:?}",
                    self.path, self.pattern
                );
            } else {
                bail!(
                    "No files found under {:?} with patterns {:?}",
                    self.path,
                    self.pattern
                );
            }
        }

        Ok(actions)
    }
}

/// Specifies a symbolic link file to be staged into the target directory.
#[derive(Clone, Debug)]
pub struct Symlink {
    /// The literal path for the target to point to.
    pub target: path::PathBuf,
    /// Specifies the name the symlink should be given.
    /// Default is the filename of the `target`.
    pub rename: String,
    pub access: Vec<Access>,
}

impl ActionBuilder for Symlink {
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error> {
        let target = self.target.as_path();
        let rename = path::Path::new(&self.rename);
        if rename.file_name() != Some(rename.as_os_str()) {
            bail!("Symlink rename must not change directories: {:?}", rename);
        }
        let staged = target_dir.join(rename);
        let link: Box<action::Action> = Box::new(action::Symlink::new(&staged, target));

        let mut actions = vec![link];
        actions.extend(self.access.iter().cloned().map(|a| {
            let a = action::Access::new(target, a.op);
            let a: Box<action::Action> = Box::new(a);
            a
        }));

        Ok(actions)
    }
}

#[derive(Clone, Debug)]
pub struct Access {
    /// Specifies  permissions to be applied to the file.
    pub op: String,
}
