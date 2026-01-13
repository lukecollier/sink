use std::{
    env,
    process::ExitCode,
    thread,
    time::{Duration, Instant},
};

use clap::{Parser, Subcommand};
use client::start_deamon;

use anyhow::Result;
use core::{is_daemon_running, objects};
use std::path::Path;

use colored::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Init,
    Open,
    Shutdown,
    Close,
}

// Space 0 will ALWAYS be the command
fn main() -> Result<ExitCode> {
    let mut hasher = seahash::SeaHasher::new();
    fn success(msg: &str) -> () {
        let prefix = "[success]".green().bold();
        println!("{prefix} {msg}")
    }
    fn error(msg: &str) -> () {
        let prefix = "[error]".red().bold();
        println!("{prefix} {msg}")
    }
    fn info(msg: &str) -> () {
        let i_prefix = "[info]".blue().bold();
        println!("{i_prefix} {msg}")
    }
    let user = std::env::var("USER")?;

    let args: Cli = Cli::parse();
    match args.command {
        Commands::Init => {
            let path = std::env::current_dir()?;
            let runtime = tokio::runtime::Builder::new_current_thread().build()?;
            let objects = runtime.block_on(core::objects::DirectoryObject::from_directory(
                &path,
                &mut hasher,
            ));
            let before = Instant::now();
            dbg!(objects?);
            let after = Instant::now();
            println!("{:?}", after - before);
            Result::Ok(ExitCode::SUCCESS)
        }
        Commands::Open => {
            if !is_daemon_running() {
                start_deamon(&user)?;
                success("starting daemon");
                // todo: need a spinner
                loop {
                    if is_daemon_running() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                success("started daemon");
            }
            info("did thing");
            Result::Ok(ExitCode::SUCCESS)
        }
        Commands::Shutdown => {
            if is_daemon_running() {
                core::messages::Command::Shutdown.send()?;
                info("shutdown sent, waiting...");
                loop {
                    if !is_daemon_running() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                success("daemon shutdown");
                // todo: We need to connect to the daemon server and await it's shutdown
                return Result::Ok(ExitCode::SUCCESS);
            } else {
                info("daemon not running, no shutdown required");
                return Result::Ok(ExitCode::FAILURE);
            }
        }
        Commands::Close => {
            if !is_daemon_running() {
                start_deamon(&user)?;
            }
            core::messages::Command::Close.send()?;
            Result::Ok(ExitCode::SUCCESS)
        }
    }
}

pub enum Command {
    Init,
    Open,
    Close,
    Compose,
    Release,
    Remove,
}

pub fn resolve_command(command: Command) -> fn() -> Result<&'static str, &'static str> {
    match command {
        Command::Open => open_command,
        _ => not_implemented,
    }
}

fn open_command() -> Result<&'static str, &'static str> {
    Ok("opened stream to blah blah")
}

fn not_implemented() -> Result<&'static str, &'static str> {
    Err("not yet been implemented")
}

fn parse_operand(command: &str) -> Result<Command, &'static str> {
    match command.to_lowercase().as_str() {
        "open" => Ok(Command::Open),
        "close" => Ok(Command::Close),
        _ => Err("not a known command"),
    }
}
