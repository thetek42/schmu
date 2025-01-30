use std::env;

use anyhow::Result;
use serde::Serialize;
use ytmapi_rs::YtMusic;
use ytmapi_rs::common::YoutubeID;

pub async fn search(query: &str) -> Result<Vec<Song>> {
    let ytm = match env::var("SCHMU_SERVER_YTAPI_COOKIE") {
        Ok(cookie) => YtMusic::from_cookie(cookie).await?,
        Err(_) => YtMusic::from_cookie_file("./cookie.txt").await?,
    };

    let songs = ytm.search_songs(query).await?;

    let result = songs
        .into_iter()
        .map(|song| Song {
            id: song.video_id.get_raw().to_owned(),
            title: song.title,
            artist: song.artist,
            thumbnail: song.thumbnails.into_iter().next().map(|x| x.url),
        })
        .collect();

    Ok(result)
}

#[derive(Serialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub thumbnail: Option<String>,
}
