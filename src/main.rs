use std::thread;
use std::time::Duration;

use state::Song;
use ui::UI;

use crate::downloader::Downloader;

mod downloader;
mod logger;
mod state;
mod ui;
mod util;

fn main() {
    logger::init();

    let ui = UI::start();

    let mut state = state::get();
    state.enqueue(Song::new("YBdyc1WDlBQ", "Livin' On A Prayer", "Bon Jovi"));
    state.enqueue(Song::new("1eQWdpWjXlk", "Glory of Love", "Peter Cetera"));
    drop(state);

    let id = "YBdyc1WDlBQ";
    let downloader = Downloader::new();
    downloader.enqueue(id);

    while ui.is_open() {
        thread::sleep(Duration::from_millis(50));
    }
}
