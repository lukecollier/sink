use std::process::ExitCode;

use anyhow::*;
use client::start_deamon;

fn main() -> Result<ExitCode> {
    let user = std::env::var("USER")?;
    start_deamon(&user)?;
    Ok(ExitCode::SUCCESS)
}
