use std::process::ExitCode;

use anyhow::*;
use client::run_client;

fn main() -> Result<ExitCode> {
    run_client()?;
    Ok(ExitCode::SUCCESS)
}
