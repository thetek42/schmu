use futures_util::{FutureExt, select};

mod connections;
mod server;
mod socket;
mod ytapi;

#[tokio::main]
async fn main() {
    shared::logger::init();

    let socket_handle = tokio::spawn(socket::start());
    let server_handle = tokio::spawn(server::start());

    select! {
        res = socket_handle.fuse() => log::error!("socket handling quit: {res:?}"),
        res = server_handle.fuse() => log::error!("server handling quit: {res:?}"),
    }
}
