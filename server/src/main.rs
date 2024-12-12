mod connections;
mod server;
mod socket;

fn main() {
    shared::logger::init();
    let socket = socket::start();
    let server = server::start();
    _ = socket.join();
    _ = server.join();
}
