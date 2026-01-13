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

#[derive(Debug)]
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
pub struct DirectoryObject {
    files: HashMap<PathBuf, FileObject>,
}

impl DirectoryObject {
    pub async fn from_directory<H: Hasher>(root_path: &Path, hasher: &mut H) -> Result<Self> {
        // todo: We would need to get all the gitignores first before we traverse all the files
        let mut ignore_builder = GitignoreBuilder::new(root_path);
        match ignore_builder.add(root_path.join(".gitignore")) {
            Some(err) => eprintln!("{err:?}"),
            None => (),
        }
        ignore_builder.add_line(None, ".git")?;
        let (matcher, _) = ignore_builder.build_global();
        let mut files = HashMap::new();
        let mut directories = vec![root_path.to_path_buf()];
        while let Some(directory_path) = directories.pop() {
            assert!(directory_path.is_dir());
            assert!(directory_path.is_absolute());
            let mut directory = fs::read_dir(directory_path).await?;
            while let Some(entry) = directory.next_entry().await? {
                let absolute_path = entry.path();
                let relative_path = absolute_path.strip_prefix(root_path)?.to_path_buf();
                if matcher
                    .matched(&relative_path, relative_path.is_dir())
                    .is_ignore()
                {
                    println!("ignoring {relative_path:?}");
                    continue;
                }
                if absolute_path.is_file() {
                    let mut file = fs::File::open(&absolute_path).await?;
                    let file_obj = FileObject::from_file(&mut file, hasher).await?;
                    files.insert(relative_path, file_obj);
                } else if absolute_path.is_dir() {
                    directories.push(absolute_path);
                }
            }
        }
        Ok(Self { files })
    }
}
