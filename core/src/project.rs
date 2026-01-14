use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    git_ignore: Option<Gitignore>,
}

// todo: We need to be able to handle subdirectory git ignores, basically as we find them we build
// a new GitIgnore with a root relative to that sub directory. We then traverse all our roots and
// when they can be stripped from a path we're inside that root (so the global gitignore will still
// work) after this we run the matches and if any are ignore then we ignore.
impl Project {
    pub fn new_global_or_default(root: &Path) -> Self {
        Self::new_global(root).unwrap_or(Self {
            root: root.to_path_buf(),
            git_ignore: None,
        })
    }

    pub fn new_global(root: &Path) -> anyhow::Result<Self> {
        let mut ignore_builder = GitignoreBuilder::new(root);
        match ignore_builder.add(root.join(".gitignore")) {
            Some(err) => eprintln!("{err:?}"),
            None => (),
        }
        ignore_builder.add_line(None, ".git")?;
        let (matcher, _) = ignore_builder.build_global();

        Ok(Self {
            root: root.to_path_buf(),
            git_ignore: Some(matcher),
        })
    }

    /// This differs from exists as it traverses backwards through the path checking if any parents
    /// don't match. if any of the parents dont match then we return None.
    pub fn exists_parent<'a>(&self, path: &'a Path, is_dir: bool) -> Option<&'a Path> {
        if let Result::Ok(relative_path) = path.strip_prefix(&self.root) {
            let Some(git_ignore) = &self.git_ignore else {
                // if no gitignore then simply return the relative path
                return Some(relative_path);
            };

            // check if any of the parent's are ignored
            let mut recursive_dir = relative_path.parent();
            while let Some(parent_path) = recursive_dir {
                // short circuit if one of our parents don't match
                if git_ignore.matched(parent_path, true).is_ignore() {
                    return None;
                }
                recursive_dir = parent_path.parent();
            }
            if git_ignore.matched(relative_path, is_dir).is_ignore() {
                // gitignored
                None
            } else {
                // found!
                Some(relative_path)
            }
        } else {
            // outside our root, ignore
            None
        }
    }

    /// None if ignored, relative path if not ignored
    pub fn exists<'a>(&self, path: &'a Path, is_dir: bool) -> Option<&'a Path> {
        if let Result::Ok(relative_path) = path.strip_prefix(&self.root) {
            println!("{path:?} -> {relative_path:?}");
            let Some(git_ignore) = &self.git_ignore else {
                // if we have a gitignore do the ignores
                return Some(relative_path);
            };
            if git_ignore.matched(relative_path, is_dir).is_ignore() {
                // gitignored
                None
            } else {
                // found!
                Some(relative_path)
            }
        } else {
            // outside our root, ignore
            None
        }
    }
}
