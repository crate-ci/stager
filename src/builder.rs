use std::collections::BTreeMap;
use std::ffi;
use std::path;

use failure;
use globwalk;

use action;

pub type Staging = BTreeMap<path::PathBuf, Vec<Box<ActionBuilder>>>;

pub trait ActionBuilder {
    // TODO(epage):
    // - Change to `Iterator`.
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error>;
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
        let filename = self.rename
            .as_ref()
            .map(|n| ffi::OsStr::new(n))
            .unwrap_or_else(|| path.file_name().unwrap_or_default());
        let copy_target = target_dir.join(filename);
        let copy: Box<action::Action> = Box::new(action::CopyFile::new(&copy_target, path));

        let mut actions = vec![copy];
        actions.extend(self.access.iter().cloned().map(|a| {
            let a = action::Access::new(target_dir, a.op);
            let a: Box<action::Action> = Box::new(a);
            a
        }));
        actions.extend(self.symlink.iter().map(|s| {
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
    ///  Specifies the root path that `patterns` will be run on to identify files to be copied into
    ///  the target directory.
    pub path: path::PathBuf,
    /// Specifies the pattern for executing the recursive/multifile match.
    pub pattern: Vec<String>,
    pub follow_links: bool,
    pub access: Vec<Access>,
}

impl ActionBuilder for SourceFiles {
    fn build(&self, target_dir: &path::Path) -> Result<Vec<Box<action::Action>>, failure::Error> {
        let mut actions: Vec<Box<action::Action>> = Vec::new();
        // TODO(epage): swap out globwalk for something that uses gitignore so we can have
        // exclusion support.
        let source_root = self.path.as_path();
        for entry in globwalk::GlobWalker::from_patterns(&self.pattern, source_root)?
            .follow_links(self.follow_links)
        {
            let entry = entry?;
            let source_file = entry.path();
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
            bail!(
                "No files found under {:?} with patterns {:?}",
                self.path,
                self.pattern
            );
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
        let staged = target_dir.join(&self.rename);
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
