use std::path::PathBuf;

pub fn audio_cache_location(id: &str) -> PathBuf {
    let mut cache = dirs::cache_dir().unwrap();
    cache.push(&format!("schmu/{id}.m4a"));
    cache
}

pub fn song_info_cache_location(id: &str) -> PathBuf {
    let mut cache = dirs::cache_dir().unwrap();
    cache.push(&format!("schmu/{id}.json"));
    cache
}

pub fn submission_url(id: &str) -> String {
    format!(
        "http://{}:{}/submit/{id}",
        shared::consts::SERVER_ADDRESS,
        shared::consts::WEBSERVER_PORT_PUBLIC,
    )
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
