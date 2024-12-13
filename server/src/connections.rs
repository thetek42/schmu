use std::sync::{Mutex, MutexGuard};

use rand::Rng;

static CONNECTIONS: Mutex<Connections> = Mutex::new(Connections::new());

pub struct Connections {
    connections: Vec<Connection>,
}

impl Connections {
    const fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    pub fn register(&mut self) -> String {
        loop {
            let id = generate_id();
            if !self.connections.iter().any(|c| c.id == id) {
                self.connections.push(Connection {
                    id: id.clone(),
                    queue: Vec::new(),
                });
                return id;
            }
        }
    }

    pub fn unregister(&mut self, id: &str) {
        if let Some(index) = self.connections.iter().position(|c| c.id == id) {
            _ = self.connections.remove(index);
        }
    }

    pub fn exists(&self, id: &str) -> bool {
        self.connections.iter().any(|c| c.id == id)
    }

    pub fn submit(&mut self, id: &str, song: &str) {
        if let Some(c) = self.connections.iter_mut().find(|c| c.id == id) {
            c.queue.push(song.to_owned())
        }
    }

    pub fn retrieve_queue(&mut self, id: &str) -> Vec<String> {
        match self.connections.iter_mut().find(|c| c.id == id) {
            Some(c) => std::mem::take(&mut c.queue),
            None => Vec::new(),
        }
    }
}

pub struct Connection {
    id: String,
    queue: Vec<String>,
}

pub fn get() -> MutexGuard<'static, Connections> {
    CONNECTIONS.lock().unwrap()
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
