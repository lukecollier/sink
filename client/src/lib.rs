use core::is_daemon_running;
use core::messages::Command;
use core::messages::CommandListener;
use core::project::Project;
use daemonize::Daemonize;
use futures::executor::block_on;
use notify::{RecursiveMode, Watcher};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use tokio::select;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio::sync::Mutex;

use core;

fn path_is_child(path: &Path, parent: &Path) -> bool {
    let mut path_ref = path.parent();
    while let Some(next_path) = path_ref {
        if next_path == parent {
            return true;
        }
        path_ref = next_path.parent();
    }
    return false;
}

fn path_is_parent(path: &Path, child: &Path) -> bool {
    let mut path_ref = child.parent();
    while let Some(next_path) = path_ref {
        if next_path == path {
            return true;
        }
        path_ref = next_path.parent();
    }
    return false;
}

// leaving user to remind me to daemonize
pub fn start_deamon(user: &str) -> anyhow::Result<()> {
    if is_daemon_running() {
        return Err(anyhow::anyhow!("Daemon is already running"));
    }
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
            // todo: Should use tokio's async stuff
            let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Event>(1000);
            // todo: unfortunately we need to do some benchmarking, I noticed we're getting a heap
            // of events when doing cargo build. A tree traversal where we don't descend using
            // .gitignore could end up being much more efficient even if we result to polling and
            // hashing. The main downside is that theres no easy way to incrementally update our
            // hashes, it would be very nifty but we'd end up having to check every single event
            // and see if the path matches which would be SLOW.
            let mut watcher =
                notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                    block_on(async {
                        tx.send(res.unwrap()).await.unwrap();
                    })
                })
                .unwrap();
            println!("server watcher running");

            let roots: Mutex<HashMap<PathBuf, Project>> = Mutex::new(HashMap::new());

            rt.block_on(async {
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("Failed to setup signal handler");
                loop {
                    select! {
                        Some(()) = sigterm.recv() => {
                            println!("[client] SIGTERM Received...");
                            break
                        },
                        Some(event) = rx.recv() => {
                            for path in &event.paths {
                                let roots = roots.lock().await;
                                for file_ignorer in roots.values() {
                                    // todo: Currently transient files (files that were created and
                                    // deleted i.e from an editor like vim) would be sent to the server. We need to detect these files and ignore them
                                    // to resolve this we can spawn an async event on file
                                    // creation, after 100ms or something if we haven't received a
                                    // delete event we send the file created. to do this we can
                                    // either maintain a Mutex of "just_created" or use a broadcast
                                    // to debounce the sending
                                    if let Some(relative_path) = file_ignorer.exists_parent(path, path.is_dir()) {
                                        println!("[client] todo: send {:?} {:?} {relative_path:?} to stream store", event.kind, event.attrs);
                                    } else {
                                        println!("[client] ignored {path:?}");
                                    }
                                }
                            }
                            // so now we go through the root's and we dictate where we send the
                            // todo: We need to add a sneaky lil .gitignore check here too
                            // to do this we somehow need to store the watcher path here... we
                            // might need to spawn a new task whenever we watch instead, womp womp.
                        }

                        Some(msg) = output.next() => {
                            match msg {
                                Command::Open {
                                    path
                                } => {
                                    // todo: we need to check if we already have the parent, if we do we
                                    // don't need to do anything else
                                    // todo: we need to check if the stream we're opening is
                                    // actually the parent of an already existing stream, if it is
                                    // we need to remove the child stream and just use the parent
                                    // stream. This design is the most resource friendly.
                                    // Todo: We should find some limit to the maximum number of
                                    // files we will track, it's not feasible to track the entire
                                    // system for example (or is it???)
                                    match watcher.watch(&path, RecursiveMode::Recursive) {
                                        Ok(_) => {
                                            let mut roots = roots.lock().await;
                                            let mut should_create = true;
                                            for root in roots.keys() {
                                                // todo: Can put all this logic in a World struct
                                                // or something
                                                // instead of FileIgnorer, we can have a Project
                                                if path_is_child(&path, root) {
                                                    // if we're a child of a root we're already
                                                    // watching do nothing!
                                                    println!("[client] {path:?} is already watched by {root:?}");
                                                    should_create = false;
                                                    break;
                                                } else if path_is_parent(&path, root) {
                                                    println!("[client] {path:?} superceded {root:?}, sending remove event");
                                                    Command::Close {
                                                        path: root.to_path_buf()
                                                    }.send().unwrap();
                                                    break;
                                                }
                                            }
                                            if should_create {
                                                println!("[client] watcher added {:?}", &path);
                                                let ignorer = Project::new_global_or_default(&path);
                                                if let Some(_) = roots.insert(path.to_path_buf(), ignorer) {
                                                    println!("[client] root added");
                                                }
                                            }
                                        },
                                        Err(problem) => eprintln!("[client] {problem:?}"),
                                    };
                                },
                                Command::Close {
                                    path
                                } => {
                                    match watcher.unwatch(&path) {
                                        Ok(_) => {
                                            println!("[client] watcher removed {:?}", &path);
                                            if let Some(_) = roots.lock().await.remove(&path) {
                                                println!("[client] root removed {path:?}");
                                            }
                                        },
                                        Err(problem) => eprintln!("[client] {problem:?}"),
                                    }
                                },
                                Command::Shutdown { caller }  => {
                                    println!("[client] shutdown request by {caller}");
                                    output.shutdown().await.unwrap();
                                    break;
                                }
                            }
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
