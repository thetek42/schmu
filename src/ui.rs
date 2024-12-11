use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use raylib::prelude::*;

use crate::state;
use crate::util::CallOnDrop;

pub struct UI {
    msg_tx: Sender<Message>,
    closed_rx: Receiver<()>,
    is_open: AtomicBool,
}

impl UI {
    pub fn start() -> Self {
        let (msg_tx, msg_rx) = mpsc::channel();
        let (closed_tx, closed_rx) = mpsc::channel();

        log::info!("starting ui");
        _ = thread::spawn(move || ui(msg_rx, closed_tx));

        Self {
            msg_tx,
            closed_rx,
            is_open: AtomicBool::new(true),
        }
    }

    pub fn is_open(&self) -> bool {
        match self.closed_rx.try_recv() {
            Ok(()) => {
                self.is_open.store(false, Ordering::SeqCst);
                false
            }
            Err(_) => self.is_open.load(Ordering::SeqCst),
        }
    }

    fn quit(&self) {
        log::info!("closing ui");
        let msg = Message::Quit;
        _ = self.msg_tx.send(msg);
    }
}

impl Drop for UI {
    fn drop(&mut self) {
        self.quit();
    }
}

enum Message {
    Quit,
}

const FONT_DATA_REGULAR: &[u8] = include_bytes!("fonts/Inter-Regular.ttf");
const FONT_SIZE_REGULAR: i32 = 24;
const FONT_DATA_BOLD: &[u8] = include_bytes!("fonts/Inter-SemiBold.ttf");
const FONT_SIZE_BOLD: i32 = 18;

fn ui(msg_rx: Receiver<Message>, closed_tx: Sender<()>) {
    let _closed_tx_guard = CallOnDrop::new(|| closed_tx.send(()));

    let (mut rl, thread) = raylib::init()
        .log_level(TraceLogLevel::LOG_WARNING)
        .size(1280, 720)
        .title("schmu")
        .resizable()
        .vsync()
        .build();

    let font_regular = rl
        .load_font_from_memory(&thread, ".ttf", FONT_DATA_REGULAR, FONT_SIZE_REGULAR, None)
        .unwrap();

    let font_bold = rl
        .load_font_from_memory(&thread, ".ttf", FONT_DATA_BOLD, FONT_SIZE_BOLD, None)
        .unwrap();

    let mut spinner = Image::gen_image_color(16 * 8 + 1, 16 * 8 + 1, Color::BLACK);
    // no draw_ring for images... :(
    spinner.draw_circle(16 * 4 + 1, 16 * 4 + 1, 14 * 4 + 1, Color::WHITE);
    spinner.draw_circle(16 * 4 + 1, 16 * 4 + 1, 10 * 4 + 1, Color::BLACK);
    spinner.draw_rectangle(0, 0, 16 * 4 + 1, 16 * 8 + 1, Color::BLACK);
    let mut spinner = rl.load_texture_from_image(&thread, &spinner).unwrap();
    spinner.gen_texture_mipmaps();
    spinner.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    while !rl.window_should_close() {
        if let Ok(msg) = msg_rx.try_recv() {
            match msg {
                Message::Quit => break,
            }
        }

        let time = rl.get_time();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        let state = state::get();

        /* queue **********************************************************************************/

        for (index, song) in state.queue().iter().enumerate() {
            let y = (index * 64 + 100) as f32;

            d.draw_rectangle(100, y as i32, 48, 48, Color::DIMGRAY);

            d.draw_text_ex(
                &font_regular,
                song.title(),
                rvec2(160, y + 2.0),
                FONT_SIZE_REGULAR as f32,
                0.0,
                Color::GAINSBORO,
            );

            if !song.downloaded() {
                let rotation = ((time % 1.0) * 360.0) as f32;
                let texture_rect = rrect(0, 0, spinner.width(), spinner.height());
                let output_rect = rrect(168, y + 34.0, 16, 16);
                let origin = rvec2(8, 8);
                d.draw_texture_pro(&spinner, texture_rect, output_rect, origin, rotation, Color::GRAY);
            }

            let offset = match song.downloaded() {
                true => 0.0,
                false => 22.0,
            };

            d.draw_text_ex(
                &font_bold,
                song.artist(),
                rvec2(160.0 + offset, y + 26.0),
                FONT_SIZE_BOLD as f32,
                0.0,
                Color::GRAY,
            );
        }
    }
}
