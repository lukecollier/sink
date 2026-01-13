use anyhow::Result;
use bytes::BytesMut;
use std::{
    collections::HashMap,
    hash::Hasher,
    path::{Path, PathBuf},
};
use tokio::fs;
use tokio::io::AsyncReadExt;

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

pub struct DirectoryObject {
    files: HashMap<PathBuf, FileObject>,
}

impl DirectoryObject {
    pub async fn from_directory<H: Hasher>(path: &Path, hasher: &mut H) -> Result<Self> {
        let mut files = HashMap::new();
        let mut directories = vec![path.to_path_buf()];
        while let Some(directory_path) = directories.pop() {
            assert!(directory_path.is_dir());
            assert!(directory_path.is_absolute());
            let mut directory = fs::read_dir(directory_path).await?;
            if let Some(entry) = directory.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    let mut file = fs::File::open(&path).await?;
                    let file_obj = FileObject::from_file(&mut file, hasher).await?;
                    files.insert(path, file_obj);
                } else if path.is_dir() {
                    directories.push(path);
                }
            }
        }
        Ok(Self { files })
    }
}
