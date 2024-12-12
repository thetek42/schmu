use std::{
    collections::{vec_deque::Iter, VecDeque},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use serde::{Deserialize, Serialize};

static STATE: Mutex<State> = Mutex::new(State::new());

pub fn get() -> MutexGuard<'static, State> {
    STATE.lock().unwrap()
}

pub struct State {
    queue: VecDeque<Song>,
    playing: Option<PlayingSong>,
}

impl State {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            playing: None,
        }
    }

    pub fn queue(&self) -> Iter<'_, Song> {
        self.queue.iter()
    }

    pub fn enqueue(&mut self, song: Song) {
        self.queue.push_back(song);
    }

    pub fn mark_downloaded(&mut self, id: &str) {
        if let Some(ref mut item) = self.queue.iter_mut().find(|item| item.id == id) {
            item.downloaded = true;
        }
    }

    pub fn playing(&self) -> Option<&PlayingSong> {
        self.playing.as_ref()
    }

    pub fn update_current_time(&mut self, total: Duration, elapsed: Duration) {
        if let Some(ref mut playing) = self.playing {
            playing.total = total;
            playing.elapsed = elapsed;
        }
    }

    pub fn get_next_song(&mut self) -> Option<String> {
        let Some(index) = self.queue.iter().position(|item| item.downloaded) else {
            self.playing = None;
            return None;
        };
        let song = self.queue.remove(index).unwrap();
        let id = song.id.clone();
        self.playing = Some(PlayingSong {
            song,
            total: Duration::from_secs(0),
            elapsed: Duration::from_secs(0),
        });
        Some(id)
    }
}

#[derive(Deserialize, Serialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub downloaded: bool,
    pub thumbnail: Vec<u8>,
}

pub struct PlayingSong {
    pub song: Song,
    pub total: Duration,
    pub elapsed: Duration,
}
