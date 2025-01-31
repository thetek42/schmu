#![feature(let_chains)]

use std::fs;
use std::io::BufRead;
use std::sync::mpsc;

use clap::Parser;
use rand::seq::SliceRandom;

use crate::cli::Cli;
use crate::connection::Connection;
use crate::downloader::Downloader;
use crate::player::Player;
use crate::ui::UI;
use crate::util::Event;

mod cli;
mod connection;
mod downloader;
mod player;
mod state;
mod ui;
mod util;

fn main() {
    shared::logger::init();

    let cli = Cli::parse();

    let fallback_playlist: Option<Vec<String>> = cli.fallback_playlist.map(|path| {
        let mut ids: Vec<String> = fs::read(path)
            .expect("failed to read fallback playlist file")
            .lines()
            .map(|s| s.unwrap())
            .filter(|s| s.len() == 11)
            .collect();
        let mut rng = rand::rng();
        ids.shuffle(&mut rng);
        ids
    });

    let (event_tx, event_rx) = mpsc::channel();

    let _connection = Connection::start(
        event_tx.clone(),
        cli.request_id,
        cli.server_address.clone(),
        cli.server_port,
    );

    let _ui = UI::start(event_tx, cli.server_address, cli.server_port);
    let downloader = Downloader::start(fallback_playlist);
    let player = Player::start();

    //downloader.enqueue("YBdyc1WDlBQ");
    //downloader.enqueue("1eQWdpWjXlk");
    //downloader.enqueue("Ucmo6hDZRSY");
    //downloader.enqueue("tIFFfP87Ooc");
    //downloader.enqueue("2509z0knTSk");
    //downloader.enqueue("y3Ov7PVHHag");
    //downloader.enqueue("63rhBxnd768");

    loop {
        match event_rx.recv().unwrap() {
            Event::UIQuit => break,
            Event::NextSong => player.next(),
            Event::TogglePause => player.toggle_pause(),
            Event::ServerHello { id } => state::get().set_connected(id),
            Event::ConnError { msg } => state::get().set_connection_error(msg),
            Event::Push { song_id } => downloader.enqueue(&song_id),
            Event::ServerClose => state::get().set_connection_error("connection closed".to_owned()),
        }
    }
}
