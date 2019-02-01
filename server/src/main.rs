use ws::listen;

fn main() {
    listen("127.0.0.1:9009", |out| {
        move |rec| {
            println!("recieved: {}", rec);
            out.send(format!("{{\"status\":\"OK\", \"recieved\":\"{}\"}}", rec))
        }
    }).expect("failed to start server");
}
