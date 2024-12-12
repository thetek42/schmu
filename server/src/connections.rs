use std::sync::{Mutex, MutexGuard};

use uuid::Uuid;

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

    pub fn register(&mut self, id: Uuid) {
        self.connections.push(Connection { id });
    }

    pub fn unregister(&mut self, id: Uuid) {
        if let Some(index) = self.connections.iter().position(|c| c.id == id) {
            _ = self.connections.remove(index);
        }
    }
}

pub struct Connection {
    id: Uuid,
}

pub fn get() -> MutexGuard<'static, Connections> {
    CONNECTIONS.lock().unwrap()
}
