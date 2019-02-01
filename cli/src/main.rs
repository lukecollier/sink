use std::env;

use client::init;

use std::path::Path;
use core;

use colored::*;

// double dash for long hand options
// single dash for short hand options
// plain text for operands
// options can have arguments
// short hand dash can be imediately followed by argument
// -- should terminate options
// options go before operands
// can be multiple operands
// options on their own can be known as 'flags'
//

// Space 0 will ALWAYS be the command
fn main() {
    core::hash_dirs(Path::new("./core"));
    let s_prefix = "[success]".green().bold();
    let e_prefix = "[error]".red().bold();

    let args: Vec<String> = env::args().collect();

    // end of the world
    let result = match parse_operand(&args[1]) {
        Ok(rs) => { 
            match resolve_command(rs)() {
                Ok(msg) => { 
                    println!("{} {}", s_prefix, msg);
                    let user = env::var("USER").expect("could not get user env variable");
                    init(user.as_str()).unwrap();
                    0
                },
                Err(msg) => { 
                    eprintln!("{} command {} has {}", e_prefix, &args[1], msg);
                    1
                }
            }
        },
        Err(msg) => { 
            eprintln!("{} {} is {}, did you mean {}?", e_prefix, &args[1], msg, "[temp]".blue());
            1
        }
    };
    ::std::process::exit(result);
}

pub enum Command {
    Init,
    Open,
    Close,
    Compose,
    Release,
    Remove
}

pub fn resolve_command(command: Command) -> fn() -> Result<&'static str, &'static str> {
    match command {
        Command::Open => open_command,
        _ => not_implemented 
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
        _ => Err("not a known command") 
    }
}
