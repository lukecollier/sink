use std::{env, process::ExitCode};

use clap::{Parser, Subcommand};
use client::start_background;

use anyhow::Result;
use core::objects;
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
    Close,
}

// Space 0 will ALWAYS be the command
fn main() -> Result<ExitCode> {
    let path = std::env::current_dir()?;
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    let mut hasher = seahash::SeaHasher::new();
    let objects = runtime.block_on(core::objects::DirectoryObject::from_directory(
        &path,
        &mut hasher,
    ));
    let s_prefix = "[success]".green().bold();
    let e_prefix = "[error]".red().bold();

    let args: Cli = Cli::parse();
    match args.command {
        Commands::Init => {
            println!("hello");
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
