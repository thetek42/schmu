use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

pub struct Downloader {
    tx: Sender<DownloaderMessage>,
}

impl Downloader {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        log::info!("starting downloader");
        _ = thread::spawn(move || downloader_thread(rx));

        Self { tx }
    }

    pub fn enqueue(&self, id: &str) {
        log::info!("enqueueing download for {id}");
        let id = id.to_owned();
        let msg = DownloaderMessage::Download { id };
        self.tx.send(msg).unwrap();
    }

    fn quit(&self) {
        log::info!("terminating downloader");
        let msg = DownloaderMessage::Quit;
        self.tx.send(msg).unwrap();
    }
}

impl Drop for Downloader {
    fn drop(&mut self) {
        self.quit();
    }
}

enum DownloaderMessage {
    Download { id: String },
    Quit,
}

struct DownloadEntry {
    id: String,
    tries_left: usize,
}

impl From<String> for DownloadEntry {
    fn from(value: String) -> Self {
        Self {
            id: value,
            tries_left: 5,
        }
    }
}

fn downloader_thread(rx: Receiver<DownloaderMessage>) {
    let mut queue = VecDeque::<DownloadEntry>::new();

    'outer: loop {
        if queue.is_empty() {
            match rx.recv().unwrap() {
                DownloaderMessage::Download { id } => queue.push_back(id.into()),
                DownloaderMessage::Quit => break,
            }
        }

        while let Ok(message) = rx.try_recv() {
            match message {
                DownloaderMessage::Download { id } => queue.push_back(id.into()),
                DownloaderMessage::Quit => break,
            }
        }

        let entry = queue.pop_front().unwrap();
        let url = get_youtube_url(&entry.id);
        let cache_file = get_cache_location(&entry.id);
        if cache_file.exists() {
            log::info!("file {} in cache, skipping download", entry.id);
            continue;
        }

        log::info!("downloading {}", entry.id);
        let mut command = Command::new("yt-dlp");
        let command = command
            .arg("--format")
            .arg("bestaudio[ext=m4a]")
            .arg("--extract-audio")
            .arg("--output")
            .arg(&cache_file)
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        log::info!("executing {command:?}");
        let mut command = command.spawn().unwrap();

        loop {
            match command.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        log::info!("{} downloaded successfully", entry.id);
                    } else {
                        log::error!(
                            "{} failed to download with exit code {status}, stderr:",
                            entry.id
                        );
                        let stderr = command.stderr.take().unwrap();
                        let stderr = BufReader::new(stderr);
                        for line in stderr.lines() {
                            log::error!(" | {}", line.unwrap());
                        }
                        match entry.tries_left {
                            0 => log::warn!(
                                "skipping download of {} due to excessive errors",
                                entry.id
                            ),
                            tries_left => queue.push_front(DownloadEntry {
                                id: entry.id,
                                tries_left: tries_left - 1,
                            }),
                        }
                    }
                    continue 'outer;
                }
                Ok(None) => {
                    while let Ok(message) = rx.try_recv() {
                        match message {
                            DownloaderMessage::Download { id } => queue.push_back(id.into()),
                            DownloaderMessage::Quit => {
                                command.kill().unwrap();
                                break 'outer;
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    log::error!("failed to wait on command: {e}");
                    match entry.tries_left {
                        0 => {
                            log::warn!("skipping download of {} due to excessive errors", entry.id)
                        }
                        tries_left => queue.push_front(DownloadEntry {
                            id: entry.id,
                            tries_left: tries_left - 1,
                        }),
                    }
                    continue 'outer;
                }
            }
        }
    }
}

fn get_cache_location(id: &str) -> PathBuf {
    let mut cache = dirs::cache_dir().unwrap();
    cache.push(&format!("schmu/{id}.m4a"));
    cache
}

fn get_youtube_url(id: &str) -> String {
    format!("https://music.youtube.com/watch?v={id}")
}
