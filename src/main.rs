use crate::downloader::Downloader;
use crate::player::Player;
use crate::ui::{Event, UI};

mod downloader;
mod logger;
mod player;
mod state;
mod ui;
mod util;

fn main() {
    logger::init();

    let ui = UI::start();

    let downloader = Downloader::start();
    downloader.enqueue("YBdyc1WDlBQ");
    downloader.enqueue("1eQWdpWjXlk");
    downloader.enqueue("Ucmo6hDZRSY");
    downloader.enqueue("tIFFfP87Ooc");
    downloader.enqueue("2509z0knTSk");
    downloader.enqueue("y3Ov7PVHHag");
    downloader.enqueue("63rhBxnd768");

    let player = Player::start();

    loop {
        match ui.wait_event() {
            Event::Quit => break,
            Event::Next => player.next(),
        }
    }
}
