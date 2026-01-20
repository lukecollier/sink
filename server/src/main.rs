use axum::extract::State;
use axum::http::StatusCode;
use axum::{Router, response::IntoResponse, routing::get};
use fastwebsockets::Frame;
use fastwebsockets::OpCode;
use fastwebsockets::upgrade;
use futures::stream::StreamExt;
use futures::{AsyncWriteExt, FutureExt};
use similar::DiffableStr;
use std::time::Duration;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    process::ExitCode,
};
use vfs::async_vfs::{AsyncMemoryFS, AsyncVfsPath};

use anyhow::*;
use core::messages::ServerMessage;
use tokio::net::*;
use tower_http::timeout::TimeoutLayer;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let root: AsyncVfsPath = AsyncMemoryFS::new().into();
    let (capture_layer, log_rx) = filebrowser::logging::capture_layer();
    tracing_subscriber::registry().with(capture_layer).init();
    let filebrowser_shutdown =
        tokio::spawn(filebrowser::start_browser(root.clone(), true, log_rx)).map(|_| ());
    let app = Router::new()
        .route("/ws", get(ws_handler).with_state(root))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            Duration::from_secs(1),
        ));
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9999));
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(filebrowser_shutdown)
        .await?;
    return Ok(ExitCode::SUCCESS);
}

async fn handle_msg(vfs_path: &mut AsyncVfsPath, msg: ServerMessage) -> Result<()> {
    match msg {
        ServerMessage::Create { path, content } => {
            if path.is_dir() {
                return Err(anyhow!("can't create a directory"));
            }
            let path = vfs_path.join(
                path.to_str()
                    .ok_or(anyhow!("path does not exist on server"))?,
            )?;
            path.parent().create_dir_all().await?;
            let mut file = path.create_file().await?;
            if let Some(content) = content {
                file.write_all(content.as_bytes()).await?;
            }
            Ok(())
        }
        ServerMessage::Delete { path } => {
            if path.is_dir() {
                return Err(anyhow!("can't delete directory"));
            }
            let path = vfs_path.join(
                path.to_str()
                    .ok_or(anyhow!("path does not exist on server"))?,
            )?;
            path.remove_file().await?;
            let mut parent = path.parent();
            while !parent.is_root() {
                let count = parent.read_dir().await?.count().await;
                if count == 0 {
                    parent.remove_dir().await?;
                } else {
                    break;
                }
                parent = parent.parent();
            }
            Ok(())
        }
        // todo: modify should specify the a range of lines that it's changed.
        // we'd then defer the commiting of the changes in the stream so we can accumulate diff's
        // from multiple clients and resolve conflicts. This means that each stream will need a
        // "projected" state for every participant, by this theres a second file structure that has
        // the "floating" changes made by participants, when a conflict occur's it'll be surfaced
        // to each client where they can resolve the issues. How this could look like is a file
        // get's marked _dirty_ when a conflict occur's, this each participant then need's to
        // resolve the issue in the conflicted area to enable the file's being synced again.
        //
        // to faciliate this we send Response's back from the server that notify client's of
        // conflicting files so that the client can notify the user. Conflict's will not auto
        // commit, the only options available to a client are to accept the other changes.
        //
        // Will require investigation, ideally modifying the same file is fine and can be handled
        // until they modify the same lines. More advanced solutions would be that when two users
        // are editing the same source file we use tree sitter to represent the file as an AST
        // which we store as a binary file along side the source file. We would need to define a
        // way for the AST to be rendered and formatted back into source code. This would side step
        // formatters on the client causing annoying conflicts, but would also mean sink handled
        // formatting by default which could be a feature, or could be annoying. Probably worth
        // using this as an OPT in if it does prove itself to work. It seem's treesitter might
        // complicate this, so for now we could support parsing the rust AST with the rust tool's
        //
        // Another approach would be to store an uglified version of the text, ie we remove all
        // whitespace, check for diff's, rustfmt, update clients with the formatted code. I don't
        // hate the idea that sink automatically formats code for client's as it gives consistent
        // formatting but could annoy some people who have their own formatting rules set up.
        // Theres also performance implications, we're sending files over the wire here so
        // communicating the new file could end up being prohibatively expensive.
        //
        // note: A fun aside for detecting desync's is that we hash every subfile, then for
        // directories we hash all the hashes recursively creating a tree that can quickly identify
        // where the differences between the server and the client occur. this would be good for
        // disconnects.
        ServerMessage::Modify { content, path } => {
            if path.is_dir() {
                return Err(anyhow!("can't create a directory"));
            }
            let path = vfs_path.join(
                path.to_str()
                    .ok_or(anyhow!("path does not exist on server"))?,
            )?;
            let mut open_file = path.create_file().await?;
            open_file.write_all(content.as_bytes()).await?;
            Ok(())
        }
        ServerMessage::Project { root } => {
            if root.is_file() {
                return Err(anyhow!("project not found"));
            }
            *vfs_path = vfs_path.join(
                root.to_str()
                    .ok_or(anyhow!("path does not exist on server"))?,
            )?;
            Ok(())
        }
    }
}

async fn handle_client(vfs: AsyncVfsPath, fut: upgrade::UpgradeFut) -> Result<()> {
    let mut ws = fastwebsockets::FragmentCollector::new(fut.await?);
    let mut current_path = vfs.clone();
    loop {
        let frame = ws.read_frame().await?;
        match frame.opcode {
            OpCode::Close => break,
            OpCode::Text | OpCode::Binary => {
                match &frame.payload {
                    fastwebsockets::Payload::Bytes(bytes_mut) => {
                        let str = bytes_mut.to_string_lossy();
                        let msg: ServerMessage = (*str).try_into()?;
                        handle_msg(&mut current_path, msg).await?;
                    }
                    _ => todo!(),
                };

                let resp = Frame::new(
                    false,
                    OpCode::Binary,
                    None,
                    fastwebsockets::Payload::Bytes("yay".into()),
                );
                ws.write_frame(resp).await?;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn ws_handler(
    ws: upgrade::IncomingUpgrade,
    State(state): State<AsyncVfsPath>,
) -> impl IntoResponse {
    let (response, fut) = ws.upgrade().unwrap();

    tokio::task::spawn(async move {
        if let Err(e) = handle_client(state, fut).await {
            tracing::error!("Error in websocket connection: {}", e);
        }
    });

    response
}
