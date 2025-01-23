use anyhow::Result;
use futures_util::{select, FutureExt, SinkExt, StreamExt};
use shared::misc::CallOnDrop;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use crate::connections;

const ADDRESS: &str = "0.0.0.0:23857";

pub async fn start() -> Result<()> {
    log::info!("starting socket handler on {ADDRESS}");
    let listener = TcpListener::bind(ADDRESS).await?;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_websocket(stream));
    }

    Ok(())
}

async fn handle_websocket(socket: TcpStream) -> Result<()> {
    let peer = socket.peer_addr().unwrap();
    log::info!("new connection from {peer}");

    let socket = tokio_tungstenite::accept_async(socket).await?;
    let (mut outgoing, mut incoming) = socket.split();

    let (id, mut song_receiver) = connections::get().await.register();
    let _unregister_guard = CallOnDrop::new(|| {
        let id = id.clone();
        tokio::spawn(async move { connections::get().await.unregister(&id) })
    });
    log::info!("assigned id {id} to {peer}");
    outgoing
        .send(Message::Text(format!("hello:{id}").into()))
        .await?;

    loop {
        select! {
            res = incoming.next().fuse() => match res {
                Some(Ok(Message::Ping(d))) => outgoing.send(Message::Pong(d)).await?,
                Some(Ok(Message::Close(_))) => {
                    log::info!("closing connection to {peer}");
                    break;
                }
                Some(Ok(_)) => (),
                Some(Err(e)) => Err(e)?,
                None => {
                    log::info!("no more message from {peer}, closing connection");
                    break;
                },
            },

            song = song_receiver.recv().fuse() => if let Some(song) = song {
                log::info!("pushing {song} to {id}");
                outgoing
                    .send(Message::Text(format!("push:{song}").into()))
                    .await?;
            }
        }
    }

    Ok(())
}
