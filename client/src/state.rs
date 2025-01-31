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
    fallback_queue: VecDeque<Song>,
    playing: Option<PlayingSong>,
    connection: ConnectionState,
}

impl State {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            fallback_queue: VecDeque::new(),
            playing: None,
            connection: ConnectionState::NotConnected,
        }
    }

    pub fn queue(&self) -> Iter<'_, Song> {
        self.queue.iter()
    }

    pub fn fallback_queue(&self) -> Iter<'_, Song> {
        self.fallback_queue.iter()
    }

    pub fn has_fallback_queue(&self) -> bool {
        self.fallback_queue.len() > 0
    }

    pub fn enqueue(&mut self, song: Song, is_fallback: bool) {
        if let Some(ref playing) = self.playing
            && playing.song.id == song.id
        {
            return;
        }

        if self.queue.iter().any(|s| s.id == song.id) {
            return;
        }

        let queue = match is_fallback {
            false => &mut self.queue,
            true => &mut self.fallback_queue,
        };

        queue.push_back(song);
    }

    pub fn mark_downloaded(&mut self, id: &str) {
        if let Some(ref mut item) = self.queue.iter_mut().find(|item| item.id == id) {
            item.downloaded = true;
        }
        if let Some(ref mut item) = self.fallback_queue.iter_mut().find(|item| item.id == id) {
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
        let song = match self.queue.iter().position(|item| item.downloaded) {
            Some(index) => self.queue.remove(index).unwrap(),
            None => match self.fallback_queue.iter().position(|item| item.downloaded) {
                Some(index) => self.fallback_queue.remove(index).unwrap(),
                None => {
                    self.playing = None;
                    return None;
                }
            },
        };
        let id = song.id.clone();
        self.playing = Some(PlayingSong {
            song,
            total: Duration::from_secs(0),
            elapsed: Duration::from_secs(0),
        });
        Some(id)
    }

    pub fn set_connected(&mut self, id: String) {
        self.connection = ConnectionState::Connected { id };
    }

    pub fn set_connection_error(&mut self, msg: String) {
        self.connection = ConnectionState::Error { msg }
    }

    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection
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

pub enum ConnectionState {
    NotConnected,
    Connected { id: String },
    Error { msg: String },
}
