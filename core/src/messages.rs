use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use anyhow::*;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::sync::oneshot::error::TryRecvError;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Deletes a file
    Delete { path: PathBuf },
    /// Creates a new file
    Create {
        path: PathBuf,
        content: Option<String>,
    },
    /// Ovewrites the file with new content
    Modify { path: PathBuf, content: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Deletes a file
    Delete { path: PathBuf },
    /// Creates a new file
    Create {
        path: PathBuf,
        content: Option<String>,
    },
    /// Ovewrites the file with new content
    Modify { path: PathBuf, content: String },
    /// Changes the root of all future operations
    Project { root: PathBuf },
}

impl TryFrom<&str> for ServerMessage {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let msg: ServerMessage = serde_json::from_str(value)?;
        Result::Ok(msg)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Open { path: PathBuf },
    Close { path: PathBuf },
    Shutdown { caller: String },
}

fn socket_path() -> PathBuf {
    let package_name = env!("CARGO_PKG_NAME");
    let mut socket_path = std::env::temp_dir();
    socket_path.push(package_name);
    socket_path.set_file_name(package_name);
    socket_path.set_extension("sock");
    socket_path
}

impl Command {
    pub fn send(&self) -> Result<()> {
        let stream = UnixStream::connect(socket_path())?;
        serde_json::to_writer(stream, self)?;
        Ok(())
    }
}

pub struct CommandListener {
    shutdown: tokio::sync::oneshot::Sender<()>,
    commands: tokio::sync::mpsc::Receiver<Command>,
}

impl CommandListener {
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.shutdown
            .send(())
            .map_err(|()| anyhow!("shutdown signal did not send"))?;
        Ok(())
    }

    pub async fn next(&mut self) -> Option<Command> {
        self.commands.recv().await
    }

    // todo: Check if a daemon exists
    pub async fn start() -> Result<Self> {
        tokio::fs::remove_file(socket_path()).await?;
        let socket = tokio::net::UnixListener::bind(socket_path())?;
        let (shutdown, mut receiver) = tokio::sync::oneshot::channel();
        let (command_sender, command_receiver) = tokio::sync::mpsc::channel(100);
        let cl = CommandListener {
            shutdown,
            commands: command_receiver,
        };
        tokio::spawn(async move {
            loop {
                match receiver.try_recv() {
                    Result::Ok(_) => break,
                    Result::Err(TryRecvError::Empty) => {}
                    err @ Result::Err(_) => err?,
                };
                let (mut stream, _) = socket.accept().await?;
                {
                    let mut buf = String::new();
                    stream.read_to_string(&mut buf).await?;
                    let command = serde_json::from_str::<Command>(&buf)?;
                    command_sender.send(command).await?;
                }
            }
            Ok(())
        });
        Ok(cl)
    }
}
