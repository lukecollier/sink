// pub mod directory {
//     use std::io::{Result, Error, ErrorKind};
//     use std::hash::Hasher;
//     use std::path::Path;
//     use std::fs;

//     use walkdir::{WalkDir, DirEntry};
//     use twox_hash::XxHash;

//     // 32,767 mac osx's max files in a directory
//     type MaxDirFiles = u16;

//     #[derive(Clone)]
//     pub struct HashTree<T> {
//         key: u64,
//         nodes: Vec<Node<T>>,
//     }

//     #[derive(Clone)]
//     pub struct Leaf<T> {
//         key: u64,
//         value: T
//     }

//     #[derive(Clone)]
//     pub enum Node<T> {
//         Dir(HashTree<T>),
//         File(Leaf<T>)
//     }

//     impl<T> HashTree<T> {
//         pub fn new() -> Self {
//             HashTree { key: calc_hash(&vec![]), nodes: vec![] }
//         }

//         pub fn read(dir: &Path) -> Self {
//             let dir_tree: Result<HashTree<u8>> = map_dir(dir);
//             Self::new()
//         }

//         fn insert(&mut self, node: Node<T>, new_key: u64) {
// 			self.key = new_key;
// 			self.nodes.insert(0, node);
//         }
//     }

//     fn is_hidden(entry: &DirEntry) -> bool {
//         entry.file_name().to_str()
//             .map(|s| s.starts_with("."))
//             .unwrap_or(false)
//     }

//     fn map_dir<T>(dir: &Path) -> Result<HashTree<T>> {
//         let walker = WalkDir::new(dir);
//         for entry in walker.into_iter() {
//             let entry = entry.unwrap();
//             if entry.path().is_dir() && is_leaf_dir(entry.path()).unwrap() {
// 				println!("{}", entry.path().display());
//                 // hash_dir(&entry.path()).unwrap();
//             } else {
// 				println!("{}", entry.path().display());
//                 // hash_dir(&entry.path()).unwrap();
//             }
//         }
//         Ok(HashTree::new())
//     }

//     fn is_leaf_dir(dir: &Path) -> Result<bool> {
//         if dir.is_dir() {
//             match fs::read_dir(dir) {
//                 Ok(read_dir) => Ok(count_dirs(read_dir) == 0),
//                 Err(e) => Err(e) 
//             }
//         } else {
//             Ok(false)
//         }
//     }

//     fn count_dirs(read_dir: fs::ReadDir) -> MaxDirFiles {
//         read_dir.map(|x| x.unwrap())
//             .filter(|x| x.path().is_dir())
//             .fold(0, |acc, _| acc + 1)
//     }

//     fn reduce_dir(dir: &Path) -> Result<Vec<u8>> {
//         let mut buf: Vec<u8> = Vec::new();
//         if dir.is_dir() {
//             for entry in fs::read_dir(dir)? {
//                 let path = entry?.path();
//                 if path.is_dir() {
//                     buf.extend_from_slice(&reduce_dir(&path)?);
//                 } else {
//                     let file_bytes: Vec<u8> = fs::read(&path)?;
//                     buf.extend_from_slice(&file_bytes);
//                 }
//             }
//             Ok(buf)
//         } else {
//             // Err(Error::new(ErrorKind::Other, "is not a directory"))
//             Ok(buf)
//         }
//     }

//     fn hash_dir(dir: &Path) -> Result<u64> {
//         match reduce_dir(&dir) {
//             Ok(bytes) => Ok(calc_hash(&bytes)),
//             Err(e) => Err(e)
//         }
//     }

//     pub fn calc_hash(bytes: &Vec<u8>) -> u64 {
//         let mut hasher = XxHash::with_seed(0);
//         hasher.write(bytes);
//         hasher.finish()
//     }

// 	#[cfg(test)]
// 	mod tests {
// 		#[test]
// 		fn it_works() {
// 		}
// 	}
// }
