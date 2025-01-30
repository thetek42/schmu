use std::fmt::Write;
use std::path::PathBuf;

pub fn audio_cache_location(id: &str) -> PathBuf {
    let mut cache = dirs::cache_dir().unwrap();
    cache.push(format!("schmu/{id}.m4a"));
    cache
}

pub fn song_info_cache_location(id: &str) -> PathBuf {
    let mut cache = dirs::cache_dir().unwrap();
    cache.push(format!("schmu/{id}.json"));
    cache
}

pub fn submission_url(id: &str, server_address: &str, server_port: u16) -> String {
    let mut s = String::new();
    match server_port {
        80 => _ = write!(s, "http://{server_address}"),
        443 => _ = write!(s, "https://{server_address}"),
        port => _ = write!(s, "http://{server_address}:{port}"),
    }
    s.push_str("/submit/");
    s.push_str(id);
    s
}

pub enum Event {
    ServerHello { id: String },
    ConnError { msg: String },
    ServerClose,
    Push { song_id: String },
    UIQuit,
    NextSong,
    TogglePause,
}
