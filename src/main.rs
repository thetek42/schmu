use std::thread;
use std::time::Duration;

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

    let downloader = Downloader::new();
    downloader.enqueue("YBdyc1WDlBQ");
    downloader.enqueue("1eQWdpWjXlk");

    while ui.is_open() {
        thread::sleep(Duration::from_millis(50));
    }
}
