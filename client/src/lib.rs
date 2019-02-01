use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::Path;
use std::sync::mpsc::Sender as TSender;

use ws;

use core;
// eventually daemonize the opened process here, for now program runs loop itself

// leaving user to remind me to daemonize
pub fn init(user: &str) -> Result<&'static str, &'static str> {
    connect("localhost", 9009);

    Ok("")
}

// all watch functionality going in here for now
fn connect(server: &str, port: u16) {
    let protocol = "ws";

    let (tx, rx) = channel();

    let url = format!("{}://{}:{}", protocol, server, port.to_string());
    let client = std::thread::spawn(move || {
        println!("Connecting to {}", url);
        ws::connect(url, |out| {
            Client {
                out: out,
                thread_out: tx.clone()
            }
        }).unwrap()
    });

    let (tx_watcher, rx_watcher) = channel();
    let mut watcher: RecommendedWatcher = 
        Watcher::new(tx_watcher.clone(), 
            Duration::from_secs(2)).expect("watcher failed to start");
    watcher.watch(Path::new("./client"), RecursiveMode::Recursive)
        .expect("cant watch path");
     if let Ok(Event::Connect(sender)) = rx.recv() {

         loop {
             if let Ok(Event::Disconnect) = rx.try_recv() {
                 eprintln!("disconnected");
                 break;
             }
             match rx_watcher.recv() {
                 Ok(event) => {
                     sender.send(format!("{:?}", event)).expect("error sending event");
                     println!("{:?}", event)
                 },
                 Err(e) => {
                     println!("{:?}", e)
                 },
             }
         }
     }

    client.join().unwrap();
}

// WebSocket connection handler for the client connection
struct Client {
    out: ws::Sender, 
    thread_out: TSender<Event>
}

impl Client {
    // Core business logic for client, keeping it DRY
    // fn event(&mut self) -> ws::Result<()> {
    //     Ok(())
    // }
}

impl ws::Handler for Client {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        self.thread_out
            .send(Event::Connect(self.out.clone()))
            .map_err(|err| {
                ws::Error::new(
                    ws::ErrorKind::Internal, 
                    format!("Unable to communicate between threads: {:?}.", err))
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
