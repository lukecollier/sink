use anyhow::{Result, anyhow};
use futures::StreamExt;
use vfs::async_vfs::AsyncVfsPath;

use crate::app::DirEntry;

/// Load directory entries, silently filtering out permission denied entries
pub async fn load_directory(path: &AsyncVfsPath) -> Result<Vec<DirEntry>> {
    let mut entries = Vec::new();

    match path.read_dir().await {
        Ok(mut dir_entries) => {
            // Collect all entries, filtering out inaccessible ones
            while let Some(entry_path) = dir_entries.next().await {
                let name = entry_path.filename();
                let is_dir = entry_path.is_dir().await.unwrap_or(false);

                entries.push(DirEntry {
                    path: entry_path,
                    name,
                    is_dir,
                });
            }
        }
        Err(e) => {
            return Err(anyhow!("Failed to read directory: {}", e));
        }
    }

    // Sort entries: directories first, then alphabetically
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}

/// Navigate to parent directory
pub async fn navigate_up(path: &AsyncVfsPath) -> Result<AsyncVfsPath> {
    // Try to get parent, otherwise return current path (at root)
    match path.parent() {
        parent => Ok(parent),
    }
}

/// Navigate into a directory
pub async fn navigate_into(path: &AsyncVfsPath) -> Result<AsyncVfsPath> {
    if !path.is_dir().await? {
        return Err(anyhow!("Path is not a directory"));
    }
    Ok(path.clone())
}

/// Recursively find an existing ancestor directory
pub async fn find_existing_ancestor(path: AsyncVfsPath) -> AsyncVfsPath {
    let mut current = path;
    loop {
        if current.is_dir().await.unwrap_or(false) {
            return current;
        }
        current = current.parent();
    }
}

/// Read file preview (first N lines, max size limit)
pub async fn read_file_preview(path: &AsyncVfsPath) -> Result<String> {
    const MAX_SIZE: usize = 100 * 1024; // 100KB max
    const MAX_LINES: usize = 100;

    if path.is_dir().await? {
        return Err(anyhow!("Cannot preview a directory"));
    }

    match path.read_to_string().await {
        Ok(content) => {
            // Limit size
            let content = if content.len() > MAX_SIZE {
                format!(
                    "{}...\n\n[File truncated - showing first {} bytes]",
                    &content[..MAX_SIZE],
                    MAX_SIZE
                )
            } else {
                content
            };

            // Limit lines
            let lines: Vec<&str> = content.lines().collect();
            let content = if lines.len() > MAX_LINES {
                format!(
                    "{}\n\n[Preview truncated - showing first {} lines]",
                    lines[..MAX_LINES].join("\n"),
                    MAX_LINES
                )
            } else {
                content
            };

            // Check if it looks like binary data
            if content
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
            {
                Ok("[Binary file - cannot display]".to_string())
            } else {
                Ok(content)
            }
        }
        Err(e) => Err(anyhow!("Failed to read file: {}", e)),
    }
}
