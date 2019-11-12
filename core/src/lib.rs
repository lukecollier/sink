mod directory;
mod hash_tree;

use twox_hash::XxHash;

use std::io::Result;
use std::hash::Hasher;
use std::path::{PathBuf, Path};
use std::fs;


fn hash_file(path: PathBuf) -> Result<u64> {
    let file_bytes = fs::read(&path)?; 
    Ok(calc_hash(&file_bytes))
}

pub fn hash_dirs(dir: &Path) -> Result<()> {
    println!("{}", dir.display());
    visit_dirs(dir, &hash_file)
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

fn calc_hash(bytes: &Vec<u8>) -> u64 {
    let mut hasher = XxHash::with_seed(0);
    hasher.write(bytes);
    hasher.finish()
}
