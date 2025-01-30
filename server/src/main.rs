mod connections;
mod server;
mod socket;
mod ytapi;

#[tokio::main]
async fn main() {
    shared::logger::init();
    let res = server::start().await;
    log::error!("server handling quit: {res:?}");
}
