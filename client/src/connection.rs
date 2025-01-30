use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Error, Message, WebSocket};

use crate::util::{self, Event};

pub struct Connection {
    msg_tx: Sender<ThreadMessage>,
    thread: Option<JoinHandle<()>>,
}

impl Connection {
    pub fn start(event_tx: Sender<Event>, request_id: Option<String>) -> Self {
        let (msg_tx, msg_rx) = mpsc::channel();

        log::info!("starting player");
        let thread = thread::spawn(move || ConnectionThread::run(msg_rx, event_tx, request_id));

        Self {
            msg_tx,
            thread: Some(thread),
        }
    }

    fn quit(&self) {
        log::info!("terminating connection");
        let msg = ThreadMessage::Quit;
        self.msg_tx.send(msg).unwrap();
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.quit();
        if let Some(thread) = self.thread.take() {
            _ = thread.join();
        }
    }
}

enum ThreadMessage {
    Quit,
}

struct ConnectionThread {
    msg_rx: Receiver<ThreadMessage>,
    event_tx: Sender<Event>,
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl ConnectionThread {
    fn run(msg_rx: Receiver<ThreadMessage>, event_tx: Sender<Event>, request_id: Option<String>) {
        let mut socket = match Self::open_socket() {
            Ok(socket) => socket,
            Err(e) => {
                log::info!("failed to connect to server: {e}");
                let msg = e.to_string();
                event_tx.send(Event::ConnError { msg }).unwrap();
                return;
            }
        };

        let msg = match request_id {
            Some(request_id) => {
                log::info!("requesting id {request_id}");
                format!("hello:{request_id}")
            }
            None => format!("hello"),
        };
        if let Err(e) = socket.send(Message::Text(msg)) {
            log::info!("failed to send hello to server: {e}");
            let msg = e.to_string();
            event_tx.send(Event::ConnError { msg }).unwrap();
            return;
        }

        let mut connection = Self {
            socket,
            msg_rx,
            event_tx,
        };

        loop {
            match connection.run_iter() {
                Ok(true) => continue,
                Ok(false) => {
                    log::error!("connection quit");
                    connection.event_tx.send(Event::ServerClose).unwrap();
                    break;
                }
                Err(e) => {
                    log::error!("connection handling failed: {e}");
                    let msg = e.to_string();
                    connection.event_tx.send(Event::ConnError { msg }).unwrap();
                    return;
                }
            }
        }
    }

    fn run_iter(&mut self) -> Result<bool> {
        match self.msg_rx.try_recv() {
            Ok(ThreadMessage::Quit) | Err(TryRecvError::Disconnected) => {
                _ = self.socket.close(None);
                return Ok(false);
            }
            Err(TryRecvError::Empty) => (),
        }

        match self.socket.read() {
            Ok(Message::Ping(d)) => self.socket.send(Message::Pong(d))?,
            Ok(Message::Close(_)) => return Ok(false),
            Ok(Message::Text(t)) => self.handle_message(&t),
            Ok(_) => (),
            Err(Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50))
            }
            Err(e) => Err(e)?,
        }

        Ok(true)
    }

    fn handle_message(&self, s: &str) {
        if s.starts_with("hello:") {
            let id = &s[6..];
            if id.len() > 0 {
                let id = id.to_owned();
                log::info!("connected with id {id}");
                log::info!("submission url: {}", util::submission_url(&id));
                self.event_tx.send(Event::ServerHello { id }).unwrap();
            }
        } else if s.starts_with("push:") {
            let song_id = &s[5..];
            if song_id.len() == 11 {
                let song_id = song_id.to_owned();
                log::info!("received new song {song_id}");
                self.event_tx.send(Event::Push { song_id }).unwrap();
            }
        }
    }

    fn open_socket() -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
        let address = format!("wss://{}:443/ws", shared::consts::SERVER_ADDRESS,);

        log::info!("{address}");

        let (socket, _) = tungstenite::connect(&address)?;

        match socket.get_ref() {
            MaybeTlsStream::Plain(socket) => socket.set_nonblocking(true)?,
            MaybeTlsStream::NativeTls(socket) => socket.get_ref().set_nonblocking(true)?,
            _ => panic!("tls not supported yet lmao"),
        }

        Ok(socket)
    }
}
