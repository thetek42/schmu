use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use shared::misc::CallOnDrop;
use tungstenite::{Error, Message};

use crate::connections;

const ADDRESS: &str = "0.0.0.0:23857";

pub fn start() -> JoinHandle<()> {
    thread::spawn(|| {
        while let Err(e) = socket_handler() {
            log::error!("socket handler failed: {e}");
            thread::sleep(Duration::from_millis(100));
        }
    })
}

fn socket_handler() -> Result<()> {
    log::info!("starting socket handler on {ADDRESS}");
    let server = TcpListener::bind(ADDRESS)?;

    let mut handles = Vec::new();

    for socket in server.incoming() {
        match socket {
            Ok(socket) => {
                let handle = thread::spawn(move || {
                    if let Err(e) = handle_websocket(socket) {
                        log::error!("error during websocket handling: {e}");
                    }
                });
                handles.push(handle);
            }
            Err(e) => log::error!("failed to accept(): {e}"),
        }
    }

    for handle in handles {
        _ = handle.join();
    }

    Ok(())
}

fn handle_websocket(socket: TcpStream) -> Result<()> {
    let peer = socket.peer_addr().unwrap();
    log::info!("new connection from {peer}");

    let mut socket = tungstenite::accept(socket)?;

    let id = connections::get().register();
    let _unregister_guard = CallOnDrop::new(|| connections::get().unregister(&id));
    log::info!("assigned id {id} to {peer}");
    socket.send(Message::Text(format!("hello:{id}")))?;

    socket.get_ref().set_nonblocking(true)?;

    loop {
        match socket.read() {
            Ok(Message::Ping(d)) => socket.send(Message::Pong(d))?,
            Ok(Message::Close(_)) => {
                log::info!("({peer}) closing connection");
                break;
            }
            Ok(_) => (),
            Err(Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => (),
            Err(e) => Err(e)?,
        }

        for song in connections::get().retrieve_queue(&id) {
            log::info!("pushing {song} to {id}");
            socket.send(Message::Text(format!("push:{song}")))?;
        }

        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}
