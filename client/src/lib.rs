use core::messages::Command;
use core::messages::CommandListener;
use daemonize::Daemonize;
use std::fs::File;
use std::fs::create_dir_all;
use std::process::exit;
use tokio::select;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;

use ws;

use core;
// leaving user to remind me to daemonize
pub fn start_deamon(user: &str) -> anyhow::Result<()> {
    let package_name = "sink";
    let mut tmp_directory = std::env::temp_dir();
    tmp_directory.push(package_name);
    match create_dir_all(&tmp_directory) {
        Ok(_) => {}
        Err(_) => {}
    }
    let mut stdout_path = tmp_directory.clone();
    stdout_path.set_file_name(package_name);
    stdout_path.set_extension("out");
    let stdout = File::create(&stdout_path)?;
    let mut stderr_path = tmp_directory.clone();
    stderr_path.set_file_name(package_name);
    stderr_path.set_extension("err");
    let stderr = File::create(&stderr_path)?;

    let pid_path = core::pid_path();
    let daemonize = Daemonize::new()
        .pid_file(pid_path) // Every method except `new` and `start`
        .chown_pid_file(false) // is optional, see `Daemonize` documentation
        .working_directory(tmp_directory) // for default behaviour.
        .user(user)
        .umask(0o027) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr); // Redirect stderr to `/tmp/daemon.err`.
    match daemonize.execute() {
        daemonize::Outcome::Child(_) => {
            println!("[client] server started...");
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()?;
            let mut output = rt.block_on(CommandListener::start())?;
            rt.block_on(async {
                // Future: can add more spawn calls, signal handling, etc.
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("Failed to setup signal handler");
                loop {
                    select! {
                        Some(()) = sigterm.recv() => {
                            println!("[client] shutting down...");
                            break
                        },
                        Some(msg) = output.next() => {
                            if let Command::Shutdown = msg {
                                println!("[client] shutdown request by cli");
                                output.shutdown().await.unwrap();
                                break;
                            } else {
                                println!("[client] received {msg:?}");
                            };
                        },
                    };
                }
            });
            println!("[client] shutting down...");
            exit(0);
        }
        daemonize::Outcome::Parent(outcome) => outcome.map(|_| ())?,
    };
    Ok(())
}
