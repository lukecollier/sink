use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

pub mod messages;
pub mod objects;
pub mod project;
pub mod watcher;

// todo: Probably be in client?
pub fn is_daemon_running() -> bool {
    match std::fs::read_to_string(pid_path()) {
        Ok(pid_str) => {
            let pid = pid_str.trim();
            std::process::Command::new("kill")
                .arg("-0")
                .arg(pid)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
        Err(_) => false,
    }
}

pub fn pid_path() -> PathBuf {
    let package_name = "sink";
    let mut pid_path = std::env::temp_dir();
    pid_path.push(package_name);
    match create_dir_all(&pid_path) {
        Ok(_) => {}
        Err(_) => {}
    }
    pid_path.set_file_name(package_name);
    pid_path.set_extension("pid");
    pid_path
}

fn path_is_child(path: &Path, parent: &Path) -> bool {
    let mut path_ref = path.parent();
    while let Some(next_path) = path_ref {
        if next_path == parent {
            return true;
        }
        path_ref = next_path.parent();
    }
    return false;
}

fn path_is_parent(path: &Path, child: &Path) -> bool {
    let mut path_ref = child.parent();
    while let Some(next_path) = path_ref {
        if next_path == path {
            return true;
        }
        path_ref = next_path.parent();
    }
    return false;
}
