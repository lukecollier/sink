mod directory;

use directory::HashTree;
use std::path::Path;

fn main() {
    let dir_tree: HashTree<u8> = HashTree::read(Path::new("./"));
}

