pub mod app;
pub mod browser;
pub mod logging;
pub mod tui;
pub mod ui;

use anyhow::Result;
use app::AppState;
use crossterm::event::{KeyCode, KeyModifiers};
use tokio::signal;
use tui::{Event, Tui};
use vfs::async_vfs::AsyncVfsPath;

/// Start an interactive file browser with the given root path
///
/// # Arguments
/// * `root` - The root directory to browse
/// * `add_quit` - If true, pressing 'q' will also quit the browser. Ctrl+C always quits.
/// * `log_rx` - Receiver for tracing log events. Create one with `logging::capture_layer()`.
pub async fn start_browser(
    root: AsyncVfsPath,
    add_quit: bool,
    mut log_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<()> {

    let mut app = AppState::new(root, add_quit).await?;

    // Set up TUI
    let mut tui = Tui::new()?;

    // Spawn signal handler
    let shutdown_fut = shutdown_signal();
    tokio::pin!(shutdown_fut);

    // Main event loop
    loop {
        // Draw current state
        tui.terminal.draw(|f| ui::draw(f, &app))?;

        // Race between event handling and shutdown signal
        tokio::select! {
            _ = &mut shutdown_fut => {
                // Shutdown signal received (Ctrl+C or SIGTERM)
                break;
            }
            Some(log_msg) = log_rx.recv() => {
                // Captured log message
                app.append_stdout(&log_msg);
            }
            event = async {
                tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    tui.event_rx.recv(),
                )
                .await
            } => {
                if let Ok(Some(event)) = event {
                    let should_quit = handle_event(&mut app, event, add_quit).await?;
                    if should_quit {
                        break;
                    }
                }
            }
        }
    }

    // Clean up
    tui.shutdown()?;
    Ok(())
}

/// Signal handler for Ctrl+C and SIGTERM
/// Returns a future that completes when a shutdown signal is received
fn shutdown_signal() -> impl std::future::Future<Output = ()> {
    async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
    }
}

async fn handle_event(app: &mut AppState, event: Event, _add_quit: bool) -> Result<bool> {
    match event {
        Event::Key(key) => match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Ok(true), // Ctrl+C to exit
            KeyCode::Char('j') => {
                app.cursor_down();
                app.update_preview().await?;
                Ok(false)
            }
            KeyCode::Char('k') => {
                app.cursor_up();
                app.update_preview().await?;
                Ok(false)
            }
            KeyCode::Char('h') => {
                app.navigate_parent().await?;
                Ok(false)
            }
            KeyCode::Char('l') => {
                // Navigate into selected directory
                if let Some(entry) = app.selected_entry() {
                    if entry.is_dir {
                        app.navigate_into_selected().await?;
                    }
                }
                Ok(false)
            }
            KeyCode::F(4) => {
                // Toggle stdout display
                app.toggle_stdout();
                Ok(false)
            }
            KeyCode::Char('r') => {
                tracing::info!("refreshing file browser");
                app.refresh().await?;
                app.update_preview().await?;
                Ok(false)
            }
            _ => Ok(false),
        },
        Event::Tick | Event::Render => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfs::async_vfs::AsyncMemoryFS;

    #[tokio::test]
    async fn test_browser_creation() {
        let fs = AsyncMemoryFS::new();
        let root: AsyncVfsPath = fs.into();

        let app = AppState::new(root, true).await;
        assert!(app.is_ok());
    }

    #[tokio::test]
    async fn test_load_directory_empty() {
        let fs = AsyncMemoryFS::new();
        let root: AsyncVfsPath = fs.into();

        let entries = browser::load_directory(&root).await;
        assert!(entries.is_ok());
        assert_eq!(entries.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_navigate_up_at_root() {
        let fs = AsyncMemoryFS::new();
        let root: AsyncVfsPath = fs.into();

        let parent = browser::navigate_up(&root).await;
        assert!(parent.is_ok());
        // Should return the same path at root
        let parent_path = parent.unwrap();
        assert_eq!(root.filename(), parent_path.filename());
    }

    #[tokio::test]
    async fn test_cursor_navigation() {
        let fs = AsyncMemoryFS::new();
        let root: AsyncVfsPath = fs.into();

        let mut app = AppState::new(root, true).await.unwrap();

        // Should start at position 0
        assert_eq!(app.cursor_position, 0);

        // Cursor down should not move when list is empty
        app.cursor_down();
        assert_eq!(app.cursor_position, 0);

        app.cursor_up();
        assert_eq!(app.cursor_position, 0);
    }
}
