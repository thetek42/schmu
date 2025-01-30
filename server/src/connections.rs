use rand::Rng;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex, MutexGuard,
};

static CONNECTIONS: Mutex<Connections> = Mutex::const_new(Connections::new());

pub struct Connections {
    connections: Vec<Connection>,
}

impl Connections {
    const fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    pub fn register(&mut self, id: Option<String>) -> (String, Receiver<String>) {
        let mut id = match id {
            Some(id) => id,
            None => generate_id(),
        };

        while self.connections.iter().any(|c| c.id == id) {
            id = generate_id();
        }

        let (sender, receiver) = channel(64);
        self.connections.push(Connection {
            id: id.clone(),
            queue: sender,
        });
        (id, receiver)
    }

    pub fn unregister(&mut self, id: &str) {
        if let Some(index) = self.connections.iter().position(|c| c.id == id) {
            _ = self.connections.remove(index);
        }
    }

    pub fn exists(&self, id: &str) -> bool {
        self.connections.iter().any(|c| c.id == id)
    }

    pub async fn submit(&mut self, id: &str, song: &str) {
        if let Some(c) = self.connections.iter_mut().find(|c| c.id == id) {
            _ = c.queue.send(song.to_owned()).await;
        }
    }
}

pub struct Connection {
    id: String,
    queue: Sender<String>,
}

pub async fn get() -> MutexGuard<'static, Connections> {
    CONNECTIONS.lock().await
}

fn generate_id() -> String {
    const N: usize = 6;
    const CHARSET: &[u8] = b"abcdeghkmnpqrswxyzACEFGHLMNPRSTWY34679";

    let mut s = String::with_capacity(N);
    let mut rng = rand::thread_rng();
    for _ in 0..N {
        let char = CHARSET[rng.gen_range(0..CHARSET.len())];
        s.push(char as char);
    }

    s
}
