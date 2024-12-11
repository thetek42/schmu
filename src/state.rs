use std::sync::{Mutex, MutexGuard};

static STATE: Mutex<State> = Mutex::new(State::new());

pub fn get() -> MutexGuard<'static, State> {
    STATE.lock().unwrap()
}

pub struct State {
    queue: Vec<Song>,
}

impl State {
    const fn new() -> Self {
        Self { queue: vec![] }
    }

    pub fn queue(&self) -> &[Song] {
        &self.queue
    }

    pub fn enqueue(&mut self, song: Song) {
        self.queue.push(song);
    }
}

pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub downloaded: bool,
}
