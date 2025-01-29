use anyhow::{Result, bail};
use futures_util::{FutureExt, SinkExt, StreamExt, select};
use shared::misc::CallOnDrop;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use crate::connections;

pub async fn start() -> Result<()> {
    let address = format!("0.0.0.0:{}", shared::consts::WEBSOCKET_PORT_SERVER);
    log::info!("starting socket handler on {address}");
    let listener = TcpListener::bind(&address).await?;

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

    let id = match incoming.next().await {
        Some(Ok(Message::Text(t))) if t == "hello" => None,
        Some(Ok(Message::Text(t))) if t.starts_with("hello:") => {
            let request_id = &t[6..];
            if is_valid_id_request(request_id) {
                bail!("invalid id request");
            } else {
                Some(request_id.to_owned())
            }
        }
        Some(Ok(_)) => bail!("invalid hello message received after connect"),
        Some(Err(e)) => bail!("error after connect: {e}"),
        None => bail!("no hello message received after connect"),
    };

    let (id, mut song_receiver) = connections::get().await.register(id);
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

fn is_valid_id_request(id: &str) -> bool {
    let valid_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    id.len() >= 4 && id.chars().any(|c| !valid_char(c))
}
