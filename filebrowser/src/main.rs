use anyhow::Result;
use vfs::async_vfs::AsyncPhysicalFS;

use filebrowser::{logging, start_browser};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging with capture layer
    let (capture_layer, log_rx) = logging::capture_layer();
    tracing_subscriber::registry()
        .with(capture_layer)
        .init();

    let root = AsyncPhysicalFS::new(std::env::current_dir()?).into();
    start_browser(root, true, log_rx).await
}
