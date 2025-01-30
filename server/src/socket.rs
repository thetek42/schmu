use anyhow::{bail, Result};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{select, FutureExt, SinkExt, StreamExt};
use shared::misc::CallOnDrop;

use crate::connections;

pub async fn handle(socket: WebSocket) {
    if let Err(e) = try_handle(socket).await {
        log::error!("error handling websocket: {e}");
    }
}

async fn try_handle(socket: WebSocket) -> Result<()> {
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
    log::info!("assigned id {id}");
    outgoing
        .send(Message::Text(format!("hello:{id}").into()))
        .await?;

    loop {
        select! {
            res = incoming.next().fuse() => match res {
                Some(Ok(Message::Ping(d))) => outgoing.send(Message::Pong(d)).await?,
                Some(Ok(Message::Close(_))) => {
                    log::info!("closing connection to {id}");
                    break;
                }
                Some(Ok(_)) => (),
                Some(Err(e)) => Err(e)?,
                None => {
                    log::info!("no more message from {id}, closing connection");
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
