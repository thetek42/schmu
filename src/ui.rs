use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use raylib::prelude::*;
use shared::misc::CallOnDrop;

use crate::state;

pub struct UI {
    msg_tx: Sender<Message>,
    event_rx: Receiver<Event>,
}

impl UI {
    pub fn start() -> Self {
        let (msg_tx, msg_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        log::info!("starting ui");
        _ = thread::spawn(move || ui(msg_rx, event_tx));

        Self { msg_tx, event_rx }
    }

    pub fn wait_event(&self) -> Event {
        match self.event_rx.recv() {
            Ok(event) => event,
            Err(_) => Event::Quit,
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

pub enum Event {
    Quit,
    Next,
    TogglePause
}

const FONT_DATA_REGULAR: &[u8] = include_bytes!("fonts/Inter-Regular.ttf");
const FONT_SIZE_REGULAR: i32 = 32;
const FONT_DATA_BOLD: &[u8] = include_bytes!("fonts/Inter-SemiBold.ttf");
const FONT_SIZE_BOLD: i32 = 22;
const FONT_DATA_LIGHT: &[u8] = include_bytes!("fonts/Inter-Light.ttf");
const FONT_SIZE_LIGHT: i32 = 64;

fn ui(msg_rx: Receiver<Message>, event_tx: Sender<Event>) {
    let _closed_tx_guard = CallOnDrop::new(|| event_tx.send(Event::Quit));

    /* raylib initialisation **********************************************************************/

    let (mut rl, thread) = raylib::init()
        .log_level(TraceLogLevel::LOG_WARNING)
        .size(1280, 720)
        .title("schmu")
        .resizable()
        .build();

    rl.set_target_fps(get_monitor_refresh_rate(get_current_monitor()) as u32);

    let charset = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~äöüÄÖÜßẞ";

    let font_regular = rl
        .load_font_from_memory(
            &thread,
            ".ttf",
            FONT_DATA_REGULAR,
            FONT_SIZE_REGULAR,
            Some(charset),
        )
        .unwrap();

    let font_bold = rl
        .load_font_from_memory(
            &thread,
            ".ttf",
            FONT_DATA_BOLD,
            FONT_SIZE_BOLD,
            Some(charset),
        )
        .unwrap();

    let font_light = rl
        .load_font_from_memory(
            &thread,
            ".ttf",
            FONT_DATA_LIGHT,
            FONT_SIZE_LIGHT,
            Some(charset),
        )
        .unwrap();

    /* textures ***********************************************************************************/

    let mut spinner = Image::gen_image_color(20 * 8 + 1, 20 * 8 + 1, Color::BLACK);
    // no draw_ring for images... :(
    spinner.draw_circle(20 * 4 + 1, 20 * 4 + 1, 18 * 4 + 1, Color::WHITE);
    spinner.draw_circle(20 * 4 + 1, 20 * 4 + 1, 14 * 4 + 1, Color::BLACK);
    spinner.draw_rectangle(0, 0, 20 * 4 + 1, 20 * 8 + 1, Color::BLACK);
    let mut spinner = rl.load_texture_from_image(&thread, &spinner).unwrap();
    spinner.gen_texture_mipmaps();
    spinner.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    let no_song_cover = Image::gen_image_color(48, 48, Color::new(20, 20, 20, 255));
    let no_song_cover = rl.load_texture_from_image(&thread, &no_song_cover).unwrap();

    let mut thumbnails = ThumbnailStore::new(&mut rl, &thread);

    /* user interface *****************************************************************************/

    while !rl.window_should_close() {
        if let Ok(msg) = msg_rx.try_recv() {
            match msg {
                Message::Quit => break,
            }
        }

        thumbnails.fetch(&mut rl, &thread);

        let time = rl.get_time();
        let screen_height = rl.get_screen_height();

        /* keypress handling **********************************************************************/

        match rl.get_key_pressed() {
            Some(KeyboardKey::KEY_N) => event_tx.send(Event::Next).unwrap(),
            Some(KeyboardKey::KEY_SPACE) => event_tx.send(Event::TogglePause).unwrap(),
            _ => (),
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        /* currently playing **********************************************************************/

        match state::get().playing() {
            Some(song) => {
                let thumbnail = thumbnails.get(&song.song.id);
                draw_thumbnail(100, 100, 196, thumbnail, &mut d);

                d.draw_text_ex(
                    &font_light,
                    &song.song.title,
                    rvec2(340, 145),
                    FONT_SIZE_LIGHT as f32,
                    0.0,
                    Color::GAINSBORO,
                );

                d.draw_text_ex(
                    &font_regular,
                    &song.song.artist,
                    rvec2(342, 215),
                    FONT_SIZE_REGULAR as f32,
                    0.0,
                    Color::GRAY,
                );

                let elapsed_ratio = song.elapsed.as_millis() as f32 / song.total.as_millis() as f32;
                d.draw_rectangle(100, 300, 196, 4, Color::new(40, 40, 40, 255));
                d.draw_rectangle(
                    100,
                    300,
                    (196.0 * elapsed_ratio) as i32,
                    4,
                    Color::STEELBLUE,
                );
            }
            None => {
                draw_thumbnail(100, 100, 196, &no_song_cover, &mut d);

                d.draw_text_ex(
                    &font_light,
                    "No song queued",
                    rvec2(340, 145),
                    FONT_SIZE_LIGHT as f32,
                    0.0,
                    Color::GRAY,
                );
            }
        }

        /* queue **********************************************************************************/

        let state = state::get();
        let mut queue = state.queue();
        let mut y = 360.0;

        for song in &mut queue {
            let thumbnail = thumbnails.get(&song.id);
            draw_thumbnail(100, y as i32, 64, thumbnail, &mut d);

            d.draw_text_ex(
                &font_regular,
                &song.title,
                rvec2(180, y + 4.0),
                FONT_SIZE_REGULAR as f32,
                0.0,
                Color::GAINSBORO,
            );

            if !song.downloaded {
                let rotation = ((time % 1.0) * 360.0) as f32;
                let texture_rect = rrect(0, 0, spinner.width(), spinner.height());
                let output_rect = rrect(190, y + 46.0, 20, 20);
                let origin = rvec2(10, 10);
                d.draw_texture_pro(
                    &spinner,
                    texture_rect,
                    output_rect,
                    origin,
                    rotation,
                    Color::GRAY,
                );
            }

            let offset = match song.downloaded {
                true => 0.0,
                false => 24.0,
            };

            d.draw_text_ex(
                &font_bold,
                &song.artist,
                rvec2(181.0 + offset, y + 36.0),
                FONT_SIZE_BOLD as f32,
                0.0,
                Color::GRAY,
            );

            y += 80.0;

            if y as i32 > screen_height - 160 {
                break;
            }
        }

        let remaining = queue.count();
        if remaining > 0 {
            let plural = if remaining == 1 { "" } else { "s" };
            d.draw_text_ex(
                &font_bold,
                &format!("{remaining} more song{plural} in queue"),
                rvec2(100, y),
                FONT_SIZE_BOLD as f32,
                0.0,
                Color::DIMGRAY,
            );
        }
        drop(state);
    }
}

struct ThumbnailStore {
    thumbnails: HashMap<String, Texture2D>,
    default: Texture2D,
}

impl ThumbnailStore {
    fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        Self {
            thumbnails: HashMap::new(),
            default: Self::default_texture(rl, thread),
        }
    }

    fn get(&mut self, id: &str) -> &Texture2D {
        self.thumbnails.get(id).unwrap_or(&self.default)
    }

    fn fetch(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        for song in state::get().queue() {
            if let Entry::Vacant(entry) = self.thumbnails.entry(song.id.to_owned()) {
                let image = Image::load_image_from_mem(".png", &song.thumbnail).unwrap();
                let mut texture = rl.load_texture_from_image(thread, &image).unwrap();
                texture.gen_texture_mipmaps();
                texture.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
                entry.insert(texture);
            }
        }
    }

    fn default_texture(rl: &mut RaylibHandle, thread: &RaylibThread) -> Texture2D {
        let image = Image::gen_image_color(48, 48, Color::new(20, 20, 20, 255));
        rl.load_texture_from_image(&thread, &image).unwrap()
    }
}

fn draw_thumbnail(x: i32, y: i32, size: i32, texture: &Texture2D, draw: &mut RaylibDrawHandle<'_>) {
    let min_side = i32::min(texture.width(), texture.height());
    let offset_x = (texture.width() - min_side) / 2;
    let offset_y = (texture.height() - min_side) / 2;
    let texture_rect = rrect(offset_x, offset_y, min_side, min_side);
    let output_rect = rrect(x, y, size, size);
    let origin = rvec2(0, 0);
    draw.draw_texture_pro(
        texture,
        texture_rect,
        output_rect,
        origin,
        0.0,
        Color::WHITE,
    );
}
