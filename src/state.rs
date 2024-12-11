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
    id: String,
    title: String,
    artist: String,
    downloaded: bool,
}

impl Song {
    pub fn new(id: &str, name: &str, artist: &str) -> Self {
        Self {
            id: id.to_owned(),
            title: name.to_owned(),
            artist: artist.to_owned(),
            downloaded: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn downloaded(&self) -> bool {
        self.downloaded
    }
}
