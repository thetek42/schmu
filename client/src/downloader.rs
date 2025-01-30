use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Cursor};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use image::{ImageFormat, ImageReader};
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::state::{self, Song};
use crate::util;

/* public api *************************************************************************************/

pub struct Downloader {
    info_tx: Sender<Message>,
    info_thread: Option<JoinHandle<()>>,
    audio_thread: Option<JoinHandle<()>>,
}

impl Downloader {
    pub fn start() -> Self {
        let (info_tx, info_rx) = mpsc::channel();
        let (audio_tx, audio_rx) = mpsc::channel();

        log::info!("starting downloader");
        let info_thread = thread::spawn(move || InfoDownloaderThread::run(info_rx, audio_tx));
        let audio_thread = thread::spawn(move || AudioDownloaderThread::run(audio_rx));

        Self {
            info_tx,
            info_thread: Some(info_thread),
            audio_thread: Some(audio_thread),
        }
    }

    pub fn enqueue(&self, id: &str) {
        log::info!("enqueueing download for {id}");
        let id = id.to_owned();
        let msg = Message::Download { id };
        self.info_tx.send(msg).unwrap();
    }

    fn quit(&self) {
        log::info!("terminating downloader");
        let msg = Message::Quit;
        self.info_tx.send(msg).unwrap();
    }
}

impl Drop for Downloader {
    fn drop(&mut self) {
        self.quit();
        if let Some(thread) = self.info_thread.take() {
            _ = thread.join();
        }
        if let Some(thread) = self.audio_thread.take() {
            _ = thread.join();
        }
    }
}

/* song info downloader ***************************************************************************/

struct InfoDownloaderThread {
    info_rx: Receiver<Message>,
    audio_tx: Sender<Message>,
    queue: VecDeque<DownloadEntry>,
}

impl InfoDownloaderThread {
    const DOWNLOAD_ATTEMPTS: usize = 3;

    fn run(info_rx: Receiver<Message>, audio_tx: Sender<Message>) {
        let mut downloader = Self {
            info_rx,
            audio_tx,
            queue: VecDeque::new(),
        };

        while downloader.run_iter() {}

        downloader.audio_tx.send(Message::Quit).unwrap();
    }

    fn run_iter(&mut self) -> bool {
        if self.queue.is_empty() {
            match self.info_rx.recv() {
                Ok(Message::Download { id }) => self.enqueue(id),
                Ok(Message::Quit) => return false,
                Err(_) => return false,
            }
        }

        loop {
            match self.info_rx.try_recv() {
                Ok(Message::Download { id }) => self.enqueue(id),
                Ok(Message::Quit) => return false,
                Err(TryRecvError::Disconnected) => return false,
                Err(TryRecvError::Empty) => break,
            }
        }

        let entry = self.dequeue().unwrap();
        if entry.is_cached() {
            log::info!("file {} in cache, skipping download", entry.id);
            match self.add_to_state_queue_from_cache(&entry) {
                Ok(()) => return true,
                Err(e) => log::error!("failed to read song info of {} from cache: {e}", entry.id),
            }
        }

        let song_info = match self.fetch_song_info(&entry.id) {
            Ok(song_info) => song_info,
            Err(e) => {
                log::error!("failed to fetch song info for {}: {e}", entry.id);
                self.requeue(entry);
                return true;
            }
        };

        if let Err(e) = self.save_to_cache(&entry, &song_info) {
            log::warn!("failed to save song info for {} to cache: {e}", entry.id);
        };

        self.add_to_state_queue(song_info);
        self.audio_tx
            .send(Message::Download { id: entry.id })
            .unwrap();

        true
    }

    fn enqueue(&mut self, id: String) {
        self.queue.push_back(DownloadEntry {
            id,
            tries_left: Self::DOWNLOAD_ATTEMPTS,
        });
    }

    fn dequeue(&mut self) -> Option<DownloadEntry> {
        self.queue.pop_front()
    }

    fn requeue(&mut self, entry: DownloadEntry) {
        match entry.tries_left {
            0 => log::warn!("skipping download of {} due to excessive errors", entry.id),
            tries_left => self.queue.push_front(DownloadEntry {
                id: entry.id,
                tries_left: tries_left - 1,
            }),
        }
    }

    fn fetch_song_info(&self, id: &str) -> Result<Song> {
        log::info!("fetching song info for {id}");

        let client = Client::new();
        let request_url = format!(
            "https://www.youtube.com/oembed?format=json&url=https://www.youtube.com/watch?v={id}"
        );
        let mut response = client
            .get(request_url)
            .send()?
            .json::<YoutubeVideoResponse>()?;

        if response.author_name.ends_with(" - Topic") {
            response
                .author_name
                .truncate(response.author_name.len() - 8);
        }
        response.thumbnail_url = response
            .thumbnail_url
            .replace("hqdefault.jpg", "maxresdefault.jpg");

        log::info!(
            "got song info for {id}: {} / {}",
            response.title,
            response.author_name
        );

        log::info!("fetching thumbnail for {id} at {}", response.thumbnail_url);

        let client = Client::new();
        let orig_thumbnail = client.get(&response.thumbnail_url).send()?.bytes()?;

        let mut thumbnail = Vec::new();
        ImageReader::new(Cursor::new(orig_thumbnail))
            .with_guessed_format()?
            .decode()?
            .write_to(&mut Cursor::new(&mut thumbnail), ImageFormat::Png)?;

        Ok(Song {
            id: id.to_owned(),
            title: response.title,
            artist: response.author_name,
            downloaded: false,
            thumbnail,
        })
    }

    fn add_to_state_queue(&self, song_info: Song) {
        let mut state = state::get();
        state.enqueue(song_info);
    }

    fn add_to_state_queue_from_cache(&self, entry: &DownloadEntry) -> Result<()> {
        let path = entry.song_info_cache_location();
        let data = fs::read(path)?;
        let mut song_info = serde_json::from_slice::<Song>(&data)?;
        song_info.downloaded = true; // ok because if song is not downloaded, we re-fetch the song info
        self.add_to_state_queue(song_info);
        Ok(())
    }

    fn save_to_cache(&self, entry: &DownloadEntry, song_info: &Song) -> Result<()> {
        let path = entry.song_info_cache_location();
        let data = serde_json::to_vec(song_info)?;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, data)?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct YoutubeVideoResponse {
    author_name: String,
    thumbnail_url: String,
    title: String,
}

/* audio downloader *******************************************************************************/

struct AudioDownloaderThread {
    rx: Receiver<Message>,
    queue: VecDeque<DownloadEntry>,
}

impl AudioDownloaderThread {
    const DOWNLOAD_ATTEMPTS: usize = 3;

    fn run(rx: Receiver<Message>) {
        let mut downloader = Self {
            rx,
            queue: VecDeque::new(),
        };

        while downloader.run_iter() {}
    }

    fn run_iter(&mut self) -> bool {
        if self.queue.is_empty() {
            match self.rx.recv() {
                Ok(Message::Download { id }) => self.enqueue(id),
                Ok(Message::Quit) => return false,
                Err(_) => return false,
            }
        }

        loop {
            match self.rx.try_recv() {
                Ok(Message::Download { id }) => self.enqueue(id),
                Ok(Message::Quit) => return false,
                Err(TryRecvError::Disconnected) => return false,
                Err(TryRecvError::Empty) => break,
            }
        }

        let entry = self.dequeue().unwrap();
        if entry.is_cached() {
            log::info!("file {} in cache, skipping download", entry.id);
            return true;
        }

        self.download(entry)
    }

    fn enqueue(&mut self, id: String) {
        self.queue.push_back(DownloadEntry {
            id,
            tries_left: Self::DOWNLOAD_ATTEMPTS,
        });
    }

    fn dequeue(&mut self) -> Option<DownloadEntry> {
        self.queue.pop_front()
    }

    fn requeue(&mut self, entry: DownloadEntry) {
        match entry.tries_left {
            0 => log::warn!("skipping download of {} due to excessive errors", entry.id),
            tries_left => self.queue.push_front(DownloadEntry {
                id: entry.id,
                tries_left: tries_left - 1,
            }),
        }
    }

    fn download(&mut self, entry: DownloadEntry) -> bool {
        log::info!("downloading {}", entry.id);

        let mut command = Command::new("yt-dlp");
        let command = command
            .arg("--format")
            .arg("bestaudio[ext=m4a]")
            .arg("--extract-audio")
            .arg("--output")
            .arg(entry.audio_cache_location())
            .arg(entry.youtube_url())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        log::info!("executing {command:?}");
        let mut command = command.spawn().unwrap();

        // TODO: check for quit message while waiting
        loop {
            match command.try_wait() {
                Ok(Some(status)) if status.success() => {
                    log::info!("{} downloaded successfully", entry.id);
                    let mut state = state::get();
                    state.mark_downloaded(&entry.id);
                    return true;
                }
                Ok(Some(status)) => {
                    self.download_failed(entry, status, &mut command);
                    return true;
                }
                Ok(None) => match self.rx.try_recv() {
                    Ok(Message::Download { id }) => self.enqueue(id),
                    Ok(Message::Quit) | Err(TryRecvError::Disconnected) => {
                        _ = command.kill();
                        return false;
                    }
                    Err(TryRecvError::Empty) => thread::sleep(Duration::from_millis(50)),
                },
                Err(e) => {
                    log::error!("failed to wait on command: {e}");
                    self.requeue(entry);
                    return true;
                }
            }
        }
    }

    fn download_failed(&mut self, entry: DownloadEntry, status: ExitStatus, command: &mut Child) {
        log::error!(
            "{} failed to download with exit code {status}, stderr:",
            entry.id
        );
        let stderr = command.stderr.take().unwrap();
        let stderr = BufReader::new(stderr);
        for line in stderr.lines() {
            log::error!(" | {}", line.unwrap());
        }
        self.requeue(entry);
    }
}

/* utilities **************************************************************************************/

enum Message {
    Download { id: String },
    Quit,
}

struct DownloadEntry {
    id: String,
    tries_left: usize,
}

impl DownloadEntry {
    fn audio_cache_location(&self) -> PathBuf {
        util::audio_cache_location(&self.id)
    }

    fn song_info_cache_location(&self) -> PathBuf {
        util::song_info_cache_location(&self.id)
    }

    fn is_cached(&self) -> bool {
        self.audio_cache_location().exists()
    }

    fn youtube_url(&self) -> String {
        format!("https://music.youtube.com/watch?v={}", self.id)
    }
}
