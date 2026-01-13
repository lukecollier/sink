use daemonize::Daemonize;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::mpsc::Sender as TSender;
use std::sync::mpsc::channel;
use std::time::Duration;
use ws::connect;

use ws;

use core;
// eventually daemonize the opened process here, for now program runs loop itself

// leaving user to remind me to daemonize
pub async fn start_background(user: &str) -> anyhow::Result<()> {
    let package_name = env!("CARGO_PKG_NAME");
    let mut tmp_directory = std::env::temp_dir();
    tmp_directory.push(package_name);
    match create_dir_all(&tmp_directory) {
        Ok(_) => {}
        Err(_) => {}
    }
    let mut stdout_path = tmp_directory.clone();
    stdout_path.set_file_name(package_name);
    stdout_path.set_extension("out");
    let stdout = File::create(stdout_path)?;
    let mut stderr_path = tmp_directory.clone();
    stderr_path.set_file_name(package_name);
    stderr_path.set_extension("err");
    let stderr = File::create(stderr_path)?;

    let mut pid_name = tmp_directory.clone();
    pid_name.set_file_name(package_name);
    pid_name.set_extension("pid");
    let daemonize: Daemonize<anyhow::Result<core::messages::CommandListener>> = Daemonize::new()
        .pid_file(pid_name) // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .working_directory(std::env::temp_dir()) // for default behaviour.
        .user(user)
        .group("daemon") // Group name
        .group(2) // or group id.
        .umask(0o027) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| {
            let rt = tokio::runtime::Builder::new_current_thread().build()?;
            let socket = rt.block_on(crate::core::messages::CommandListener::start())?;
            Ok(socket)
        });
    match daemonize.start() {
        Result::Ok(Result::Ok(mut socket)) => socket.next(),
        Result::Ok(Result::Err(err)) => {
            return Result::Err(err.into());
        }
        Result::Err(err) => {
            return Result::Err(err.into());
        }
    };
    Ok(())
}

// all watch functionality going in here for now
fn start_local_server(server: &str, port: u16) {}

// all watch functionality going in here for now
// fn connect(server: &str, port: u16) {
//     let protocol = "ws";

//     let (tx, rx) = channel();

//     let url = format!("{}://{}:{}", protocol, server, port.to_string());
//     let client = std::thread::spawn(move || {
//         println!("Connecting to {}", url);
//         ws::connect(url, |out| Client {
//             out: out,
//             thread_out: tx.clone(),
//         })
//         .unwrap()
//     });

//     let (tx_watcher, rx_watcher) = channel();
//     let mut watcher: RecommendedWatcher =
//         Watcher::new(tx_watcher.clone(), Duration::from_secs(2)).expect("watcher failed to start");
//     watcher
//         .watch(Path::new("./client"), RecursiveMode::Recursive)
//         .expect("cant watch path");
//     if let Ok(Event::Connect(sender)) = rx.recv() {
//         loop {
//             if let Ok(Event::Disconnect) = rx.try_recv() {
//                 eprintln!("disconnected");
//                 break;
//             }
//             match rx_watcher.recv() {
//                 Ok(event) => {
//                     sender
//                         .send(format!("{:?}", event))
//                         .expect("error sending event");
//                     println!("{:?}", event)
//                 }
//                 Err(e) => {
//                     println!("{:?}", e)
//                 }
//             }
//         }
//     }

//     client.join().unwrap();
// }

// WebSocket connection handler for the client connection
struct Client {
    out: ws::Sender,
    thread_out: TSender<Event>,
}

impl Client {}

impl ws::Handler for Client {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        self.thread_out
            .send(Event::Connect(self.out.clone()))
            .map_err(|err| {
                ws::Error::new(
                    ws::ErrorKind::Internal,
                    format!("Unable to communicate between threads: {:?}.", err),
                )
            })
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        println!("{:?}", msg);
        Ok(())
    }

    fn on_close(&mut self, _code: ws::CloseCode, _reason: &str) {
        if let Err(err) = self.thread_out.send(Event::Disconnect) {
            eprintln!("disconnected {:?}", err)
        }
    }
}

enum Event {
    Connect(ws::Sender),
    Disconnect,
}
