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
