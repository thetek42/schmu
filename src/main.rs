use std::thread;
use std::time::Duration;

use player::Player;
use ui::UI;

use crate::downloader::Downloader;

mod downloader;
mod logger;
mod player;
mod state;
mod ui;
mod util;

fn main() {
    logger::init();

    let ui = UI::start();

    let downloader = Downloader::new();
    downloader.enqueue("YBdyc1WDlBQ");
    downloader.enqueue("1eQWdpWjXlk");
    downloader.enqueue("Ucmo6hDZRSY");
    downloader.enqueue("tIFFfP87Ooc");
    downloader.enqueue("2509z0knTSk");
    downloader.enqueue("y3Ov7PVHHag");
    downloader.enqueue("63rhBxnd768");

    let _player = Player::new();

    while ui.is_open() {
        thread::sleep(Duration::from_millis(50));
    }
}
