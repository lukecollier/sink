use twox_hash::XxHash;

use std::io::{Result, Error, ErrorKind};
use std::hash::Hasher;
use std::path::{PathBuf, Path};
use std::fs;

use walkdir::{WalkDir, DirEntry};

// 32,767 mac osx's max files in a directory
type MaxDirFiles = u16;

pub struct HashTree<T> {
    key: u64,
    nodes: Vec<Node<T>>,
}

pub enum Node<T> {
    Node(HashTree<T>),
    Leaf(T),
}

impl<T> HashTree<T> {
    pub fn new() -> Self {
        let vec = vec![];
        let vec2: Vec<Node<T>> = vec![];
        HashTree { key: calc_hash(&vec), nodes: vec2 }
    }

    pub fn read(dir: &Path) -> Self {
        let dir_tree: Result<HashTree<u8>> = map_dir(dir);
        Self::new()
    }
}

fn visit_dirs(dir: &Path, cb: &Fn(PathBuf) -> Result<u64>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(path.to_path_buf())?;
            }
        }
    }
    Ok(())
}

fn map_dir<T>(dir: &Path) -> Result<HashTree<T>> {
    for entry in WalkDir::new(dir).contents_first(true) {
        let entry = entry.unwrap();
        if entry.path().is_dir() && is_leaf_dir(entry.path()).unwrap() {
            println!("{} is leaf {} with hash directory {}", entry.path().display(), 
                     is_leaf_dir(entry.path()).unwrap(),
                     calc_hash(&reduce_dir(entry.path()).unwrap()));
        } else {
            println!("{} is leaf {}", entry.path()
                     .display(), is_leaf_dir(entry.path()).unwrap());
        }
    }

    Ok(HashTree::new())
}

fn is_leaf_dir(dir: &Path) -> Result<bool> {
    if dir.is_dir() {
        match fs::read_dir(dir) {
            Ok(read_dir) => Ok(count_dirs(read_dir) == 0),
            Err(e) => Err(e) 
        }
    } else {
        Ok(false)
    }
}

fn count_dirs(read_dir: fs::ReadDir) -> MaxDirFiles {
    read_dir.map(|x| x.unwrap())
        .filter(|x| x.path().is_dir())
        .fold(0, |acc, _| acc + 1)
}


pub fn hash_file(path: PathBuf) -> Result<u64> {
    let file_bytes = fs::read(&path)?; 
    Ok(calc_hash(&file_bytes))
}


fn reduce_dir(dir: &Path) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                buf.extend_from_slice(&reduce_dir(&path)?);
            } else {
                let file_bytes: Vec<u8> = fs::read(&path)?;
                buf.extend_from_slice(&file_bytes);
            }
        }
    }
    Ok(buf)
}

fn hash_dir(dir: &Path) -> Result<u64> {
    let file_name: &str = dir.file_name().unwrap().to_str().unwrap();
    let mut buf: Vec<u8> = Vec::new(); 
    buf.extend_from_slice(file_name.as_bytes());
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                hash_dir(&path)?;
            } else {
                let file_bytes: Vec<u8> = fs::read(&path)?;
                buf.extend_from_slice(&file_bytes);
            }
        }
    }
    Ok(calc_hash(&buf))
}

pub fn hash_dirs(dir: &Path) -> Result<()> {
    println!("{}", dir.display());
    visit_dirs(dir, &hash_file)
}

pub fn calc_hash(bytes: &Vec<u8>) -> u64 {
    let mut hasher = XxHash::with_seed(0);
    hasher.write(bytes);
    hasher.finish()
}
