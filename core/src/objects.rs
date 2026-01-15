use anyhow::Result;
use std::collections::HashSet;
use std::{
    collections::HashMap,
    hash::Hasher,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tokio::fs;
use tokio::io::AsyncReadExt;

use crate::project::Project;

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone, Copy)]
pub struct FileObject {
    /// A file objects hash
    hash: u64,
}

impl FileObject {
    async fn from_file<H: Hasher>(file: &mut fs::File, hasher: &mut H) -> anyhow::Result<Self> {
        let mut buf = Vec::new();
        loop {
            let outcome = file.read_buf(&mut buf).await?;
            hasher.write(&buf);
            if outcome == 0 {
                break;
            }
        }
        let hash = hasher.finish();
        Ok(Self { hash })
    }
}

#[derive(Debug)]
pub struct ObjectsDelta {
    pub added: HashMap<PathBuf, FileObject>,
    pub removed: HashMap<PathBuf, FileObject>,
    pub modified: HashMap<PathBuf, FileObject>,
}

impl ObjectsDelta {
    fn new() -> Self {
        Self {
            added: HashMap::new(),
            removed: HashMap::new(),
            modified: HashMap::new(),
        }
    }

    pub fn is_different(&self) -> bool {
        !self.added.is_empty() || !self.removed.is_empty() || !self.modified.is_empty()
    }

    fn add(&mut self, path: PathBuf, object: FileObject) {
        self.added.insert(path, object);
    }
    fn remove(&mut self, path: PathBuf, object: FileObject) {
        self.removed.insert(path, object);
    }
    fn modify(&mut self, path: PathBuf, object: FileObject) {
        self.modified.insert(path, object);
    }
}

#[derive(Debug)]
pub struct Objects {
    project: Project,
    pub objects: HashMap<PathBuf, FileObject>,
}

impl Objects {
    pub fn patch<'a>(&mut self, diff: ObjectsDelta) -> anyhow::Result<()> {
        for (k, v) in diff.added {
            self.objects.insert(k.to_path_buf(), v);
        }
        for (k, _) in diff.removed {
            self.objects.remove(&k);
        }
        for (k, nv) in diff.modified {
            if let Some(value) = self.objects.get_mut(&k) {
                *value = nv;
            }
        }
        Ok(())
    }
    pub async fn update<H: Hasher + Default>(
        &mut self,
        check_after: SystemTime,
    ) -> anyhow::Result<SystemTime> {
        let mut last_time = check_after.clone();
        let mut found_files = HashSet::new();
        let mut directories = vec![self.project.root.clone()];
        while let Some(directory_path) = directories.pop() {
            assert!(directory_path.is_dir());
            assert!(directory_path.is_absolute());
            let mut directory = fs::read_dir(directory_path).await?;
            while let Some(entry) = directory.next_entry().await? {
                let absolute_path = entry.path();
                let Some(relative_path) =
                    self.project.exists(&absolute_path, absolute_path.is_dir())
                else {
                    continue;
                };
                if absolute_path.is_file() {
                    let key = relative_path.to_path_buf();
                    found_files.insert(key.clone());
                    let meta = fs::metadata(&absolute_path).await?;
                    let modified_at = meta.modified()?;
                    if modified_at > check_after {
                        last_time = last_time.max(modified_at);
                        let mut file = fs::File::open(&absolute_path).await?;
                        let file_obj = FileObject::from_file(&mut file, &mut H::default()).await?;
                        self.objects.insert(key, file_obj);
                    }
                } else if absolute_path.is_dir() {
                    directories.push(absolute_path);
                }
            }
        }
        for key in self
            .objects
            .keys()
            .map(|path| path.to_path_buf())
            .collect::<HashSet<_>>()
            .symmetric_difference(&found_files)
        {
            self.objects.remove(key);
        }
        Ok(last_time)
    }

    pub async fn from_directory<H: Hasher + Default>(root_path: &Path) -> Result<Self> {
        // todo: We would need to get all the gitignores first before we traverse all the files
        let project = Project::new_global(root_path)?;
        let mut files = HashMap::new();
        let mut directories = vec![root_path.to_path_buf()];
        while let Some(directory_path) = directories.pop() {
            assert!(directory_path.is_dir());
            assert!(directory_path.is_absolute());
            let mut directory = fs::read_dir(directory_path).await?;
            while let Some(entry) = directory.next_entry().await? {
                let absolute_path = entry.path();
                let Some(relative_path) = project.exists(&absolute_path, absolute_path.is_dir())
                else {
                    continue;
                };
                if absolute_path.is_file() {
                    let mut file = fs::File::open(&absolute_path).await?;
                    let file_obj = FileObject::from_file(&mut file, &mut H::default()).await?;
                    files.insert(relative_path.to_path_buf(), file_obj);
                } else if absolute_path.is_dir() {
                    directories.push(absolute_path);
                }
            }
        }
        Ok(Self {
            objects: files,
            project,
        })
    }

    pub fn diff(&self, other: &Self) -> ObjectsDelta {
        let mut diff = ObjectsDelta::new();
        for (key, value) in &self.objects {
            if let Some(obj) = other.objects.get(key) {
                if obj.hash != value.hash {
                    diff.modify(key.to_path_buf(), *obj);
                }
            } else {
                diff.remove(key.to_path_buf(), *value);
            }
        }
        for (key, value) in &other.objects {
            if !self.objects.contains_key(key) {
                diff.add(key.to_path_buf(), *value);
            }
        }
        diff
    }
}
