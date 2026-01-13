# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Sink** is a file synchronization tool designed to work across systems with a minimal, fire-and-forget API. It's a Rust monorepo containing four main components:

- **cli**: Command-line interface for user interaction
- **core**: Shared messaging and file object abstractions (file hashing, directory traversal)
- **server**: WebSocket server that handles communication between clients
- **client**: Daemon process that watches files and syncs to the server

The project uses Unix sockets for IPC between the CLI and the daemon, and WebSockets for server-to-client communication.

## Common Commands

### Build
```bash
cargo build          # Build all packages
cargo build -p cli   # Build specific package (cli, core, server, client)
cargo build --release
```

### Run
```bash
cargo run -p cli -- <COMMAND>     # Run CLI with subcommand
cargo run -p server               # Run WebSocket server (listens on 127.0.0.1:9009)
```

### Test
```bash
cargo test           # Run all tests
cargo test -p core   # Test specific package
```

### Lint & Format
```bash
cargo fmt --all          # Format all code
cargo fmt --all -- --check  # Check formatting without changes
cargo clippy --all       # Run linter
```

## Architecture

### IPC Layer (Unix Sockets)
The daemon uses Unix sockets for inter-process communication. The socket path is generated in `core/src/messages.rs:socket_path()` using the package name. Commands (Open, Close, Shutdown) are serialized as JSON and sent to the daemon.

**Key classes:**
- `core::messages::Command`: Serializable enum for CLI-to-daemon communication
- `core::messages::CommandListener`: Server-side listener that accepts and processes commands via Unix sockets

### File Hashing & Objects
File synchronization is built on a hashing system that tracks file contents and directory structure.

**Key classes:**
- `core::objects::FileObject`: Represents a file with its content hash
- `core::objects::DirectoryObject`: Recursively scans directories and builds a HashMap of FileObjects

The CLI uses `seahash::SeaHasher` to compute hashes of the current working directory.

### Daemon & File Watching
The client daemon (`client/src/lib.rs`) daemonizes using the `daemonize` crate and:
1. Creates a CommandListener socket to receive commands from the CLI
2. Sets up file watchers (via `notify` crate) to detect changes
3. Establishes WebSocket connections to the server to sync changes

**Key function:**
- `client::start_background()`: Spawns a daemon process with configurable stdout/stderr/pid file paths

### Server
A simple echo-based WebSocket server that broadcasts received messages. Currently in proof-of-concept stage.

## Workspace Structure

```
sink/
├── cli/              # CLI entry point (clap parser, command routing)
├── core/             # Shared types (messages, file objects)
├── server/           # WebSocket server
├── client/           # Daemon and file watcher
└── Cargo.toml        # Workspace manifest
```

## Edition & Dependencies

- **Rust Edition**: 2024 (non-standard, likely a typo for 2021)
- **Async Runtime**: Tokio (selective features enabled per package)
- **Serialization**: serde + serde_json
- **CLI**: clap (derive macros)
- **File Watching**: notify crate
- **WebSockets**: ws crate (not tokio-tungstenite)
- **Hashing**: seahash and twox-hash
- **IPC**: interprocess and Unix sockets via tokio-net

## Notes for Development

- The server is currently a proof-of-concept WebSocket echo server
- The CLI in `cli/src/main.rs` has incomplete command handling; only `Init` is partially implemented
- File watching code in `client/src/lib.rs` is mostly commented out—this is where stream synchronization will be implemented
- The daemon spawning uses `daemonize` crate which requires setting user/group
- Unix sockets are cleaned up in `/tmp/` with the pattern `{package_name}.sock`
