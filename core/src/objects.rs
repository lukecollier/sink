use anyhow::{Result, anyhow};
use bytes::BytesMut;
use ignore::gitignore::GitignoreBuilder;
use std::{
    collections::HashMap,
    hash::Hasher,
    path::{Path, PathBuf},
};
use tokio::fs;
use tokio::io::AsyncReadExt;

use crate::project::{self, Project};

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct FileObject {
    /// A file objects hash
    hash: u64,
}

impl FileObject {
    async fn from_file<H: Hasher>(file: &mut fs::File, hasher: &mut H) -> anyhow::Result<Self> {
        let mut buf = BytesMut::new();
        file.read_buf(&mut buf).await?;
        hasher.write(&buf);
        let hash = hasher.finish();
        Ok(Self { hash })
    }
}

#[derive(Debug)]
pub struct ObjectsDelta<'a> {
    pub added: HashMap<&'a PathBuf, &'a FileObject>,
    pub removed: HashMap<&'a PathBuf, &'a FileObject>,
    pub modified: HashMap<&'a PathBuf, &'a FileObject>,
}

impl<'a> ObjectsDelta<'a> {
    fn new() -> Self {
        Self {
            added: HashMap::new(),
            removed: HashMap::new(),
            modified: HashMap::new(),
        }
    }

    fn add(&mut self, path: &'a PathBuf, object: &'a FileObject) {
        self.added.insert(path, object);
    }
    fn remove(&mut self, path: &'a PathBuf, object: &'a FileObject) {
        self.removed.insert(path, object);
    }
    fn modify(&mut self, path: &'a PathBuf, object: &'a FileObject) {
        self.modified.insert(path, object);
    }
}

#[derive(Debug)]
pub struct Objects {
    objects: HashMap<PathBuf, FileObject>,
}

impl Objects {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
        }
    }
    pub async fn from_directory<H: Hasher>(root_path: &Path, hasher: &mut H) -> Result<Self> {
        // todo: We would need to get all the gitignores first before we traverse all the files
        let ignorer = Project::new_global(root_path)?;
        let mut files = HashMap::new();
        let mut directories = vec![root_path.to_path_buf()];
        while let Some(directory_path) = directories.pop() {
            assert!(directory_path.is_dir());
            assert!(directory_path.is_absolute());
            let mut directory = fs::read_dir(directory_path).await?;
            while let Some(entry) = directory.next_entry().await? {
                let absolute_path = entry.path();
                let Some(relative_path) = ignorer.exists(&absolute_path, absolute_path.is_dir())
                else {
                    continue;
                };
                if absolute_path.is_file() {
                    let mut file = fs::File::open(&absolute_path).await?;
                    let file_obj = FileObject::from_file(&mut file, hasher).await?;
                    files.insert(relative_path.to_path_buf(), file_obj);
                } else if absolute_path.is_dir() {
                    directories.push(absolute_path);
                }
            }
        }
        Ok(Self { objects: files })
    }

    /// Diff's the two, note: for content diffing we'll do this in the watcher and probably leverage
    /// burnt sushi's bstr, has added, deleted, removed stored here.
    pub fn diff<'a>(&'a self, other: &'a Self) -> ObjectsDelta<'a> {
        let mut diff = ObjectsDelta::new();
        for (key, value) in &self.objects {
            if let Some(obj) = other.objects.get(key) {
                if obj != value {
                    diff.modify(key, obj);
                }
            } else {
                diff.remove(key, value);
            }
        }
        for (key, value) in &other.objects {
            if !self.objects.contains_key(key) {
                diff.add(key, value);
            }
        }
        diff
    }
}
