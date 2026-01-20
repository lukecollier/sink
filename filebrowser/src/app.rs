use anyhow::Result;
use vfs::async_vfs::AsyncVfsPath;

use crate::browser;

#[derive(Clone, Debug)]
pub struct DirEntry {
    pub path: AsyncVfsPath,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub enum PreviewContent {
    Directory(Vec<DirEntry>),
    File(String),
    Loading,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct OutputLine {
    pub text: String,
    pub is_stderr: bool,
}

pub struct AppState {
    pub current_path: AsyncVfsPath,
    pub parent_entries: Option<Vec<DirEntry>>,
    pub current_entries: Vec<DirEntry>,
    pub preview_content: PreviewContent,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub error_message: Option<String>,
    pub add_quit: bool,
    pub show_stdout: bool,
    pub stdout_output: Vec<OutputLine>,
}

impl AppState {
    pub async fn new(root: AsyncVfsPath, add_quit: bool) -> Result<Self> {
        let mut app = AppState {
            current_path: root,
            parent_entries: None,
            current_entries: vec![],
            preview_content: PreviewContent::Loading,
            cursor_position: 0,
            scroll_offset: 0,
            error_message: None,
            add_quit,
            show_stdout: false,
            stdout_output: vec![],
        };

        // Load initial directory
        app.refresh().await?;

        Ok(app)
    }

    pub async fn refresh(&mut self) -> Result<()> {
        // Load current directory entries
        match browser::load_directory(&self.current_path).await {
            Ok(entries) => {
                self.current_entries = entries;
                self.cursor_position = 0;
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(e.to_string());
                return Err(e);
            }
        }

        // Load parent directory entries if not at root
        let parent_path = self.current_path.parent();
        if parent_path.filename() != self.current_path.filename() {
            match browser::load_directory(&parent_path).await {
                Ok(entries) => {
                    self.parent_entries = Some(entries);
                }
                Err(_) => {
                    // Silently ignore errors loading parent
                    self.parent_entries = None;
                }
            }
        } else {
            self.parent_entries = None;
        }

        Ok(())
    }

    pub async fn update_preview(&mut self) -> Result<()> {
        // Update preview based on selected entry
        if let Some(entry) = self.selected_entry() {
            if entry.is_dir {
                // For directories, show directory listing
                match browser::load_directory(&entry.path).await {
                    Ok(entries) => {
                        self.preview_content = PreviewContent::Directory(entries);
                    }
                    Err(e) => {
                        self.preview_content = PreviewContent::Error(e.to_string());
                    }
                }
            } else {
                // For files, load content
                match browser::read_file_preview(&entry.path).await {
                    Ok(content) => {
                        self.preview_content = PreviewContent::File(content);
                    }
                    Err(e) => {
                        self.preview_content = PreviewContent::Error(e.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn toggle_stdout(&mut self) {
        self.show_stdout = !self.show_stdout;
    }

    pub fn append_stdout(&mut self, message: &str) {
        self.stdout_output.push(OutputLine {
            text: message.to_string(),
            is_stderr: false,
        });
    }

    pub fn append_stderr(&mut self, message: &str) {
        self.stdout_output.push(OutputLine {
            text: message.to_string(),
            is_stderr: true,
        });
    }

    pub fn clear_stdout(&mut self) {
        self.stdout_output.clear();
    }

    pub fn cursor_up(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            // Adjust scroll offset if needed
            if self.cursor_position < self.scroll_offset {
                self.scroll_offset = self.cursor_position;
            }
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor_position < self.current_entries.len().saturating_sub(1) {
            self.cursor_position += 1;
            // Adjust scroll offset if needed (will implement with UI height)
        }
    }

    pub async fn navigate_parent(&mut self) -> Result<()> {
        let parent = browser::navigate_up(&self.current_path).await?;
        if parent.filename() != self.current_path.filename() {
            self.current_path = parent;
            self.refresh().await?;
            self.update_preview().await?;
        }
        Ok(())
    }

    pub async fn navigate_into_selected(&mut self) -> Result<()> {
        if let Some(entry) = self.selected_entry() {
            if entry.is_dir {
                self.current_path = browser::navigate_into(&entry.path).await?;
                self.refresh().await?;
                self.update_preview().await?;
            }
        }
        Ok(())
    }

    pub fn is_at_root(&self) -> bool {
        let parent = self.current_path.parent();
        parent.filename() == self.current_path.filename()
    }

    pub fn selected_entry(&self) -> Option<&DirEntry> {
        self.current_entries.get(self.cursor_position)
    }
}
