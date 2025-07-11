use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use image::{ImageFormat, Luma};
use qrcode::QrCode;
use raylib::prelude::*;
use shared::misc::CallOnDrop;

use crate::state::{self, ConnectionState};
use crate::util::{self, Event};

pub struct UI {
    msg_tx: Sender<Message>,
    thread: Option<JoinHandle<()>>,
}

impl UI {
    pub fn start(event_tx: Sender<Event>, server_address: String, server_port: u16) -> Self {
        let (msg_tx, msg_rx) = mpsc::channel();

        log::info!("starting ui");
        let thread = thread::spawn(move || ui(msg_rx, event_tx, server_address, server_port));

        Self {
            msg_tx,
            thread: Some(thread),
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
        if let Some(thread) = self.thread.take() {
            _ = thread.join();
        }
    }
}

enum Message {
    Quit,
}

const FONT_DATA_REGULAR: &[u8] = include_bytes!("fonts/Inter-Regular.ttf");
const FONT_SIZE_REGULAR: i32 = 32;
const FONT_DATA_BOLD: &[u8] = include_bytes!("fonts/Inter-SemiBold.ttf");
const FONT_SIZE_BOLD: i32 = 22;
const FONT_DATA_LIGHT: &[u8] = include_bytes!("fonts/Inter-Light.ttf");
const FONT_SIZE_LIGHT: i32 = 64;

fn ui(
    msg_rx: Receiver<Message>,
    event_tx: Sender<Event>,
    server_address: String,
    server_port: u16,
) {
    let _closed_tx_guard = CallOnDrop::new(|| event_tx.send(Event::UIQuit));

    let mut queue_edit_mode: Option<usize> = None;

    /* raylib initialisation **********************************************************************/

    let (mut rl, thread) = raylib::init()
        .log_level(TraceLogLevel::LOG_WARNING)
        .size(1280, 720)
        .title("schmu")
        .resizable()
        .build();

    rl.set_exit_key(None);
    rl.set_target_fps(get_monitor_refresh_rate(get_current_monitor()) as u32);

    let charset = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~äöüÄÖÜßẞÀÁÂÃÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕØÙÚÛÝÞàáâãåæçèéêëìíîïðñòóôõøùúûýþÿ";

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

    let mut server_qrcode: Option<Texture2D> = None;
    let mut server_qrcode_id: String = "".to_owned();

    let mut thumbnails = ThumbnailStore::new(&mut rl, &thread);

    let mut qr_contrast: u8 = 105;
    let mut qr_size: u8 = 6;

    /* user interface *****************************************************************************/

    while !rl.window_should_close() {
        if let Ok(msg) = msg_rx.try_recv() {
            match msg {
                Message::Quit => break,
            }
        }

        thumbnails.fetch(&mut rl, &thread);

        if let ConnectionState::Connected { id } = state::get().connection_state() {
            if server_qrcode.is_none() || &server_qrcode_id != id {
                generate_qr_texture(
                    &mut rl,
                    &thread,
                    &mut server_qrcode,
                    &util::submission_url(id, &server_address, server_port),
                );
                server_qrcode_id = id.to_owned();
            }
        }

        let time = rl.get_time();
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();

        /* keypress handling **********************************************************************/

        if let Some(edit_index) = queue_edit_mode {
            match rl.get_key_pressed() {
                Some(KeyboardKey::KEY_ESCAPE) => queue_edit_mode = None,
                Some(KeyboardKey::KEY_D) => {
                    state::get().delete_song(edit_index);
                    queue_edit_mode = None;
                }
                Some(KeyboardKey::KEY_J) => {
                    let new_index = state::get().move_down(edit_index);
                    queue_edit_mode = Some(new_index);
                }
                Some(KeyboardKey::KEY_K) => {
                    let new_index = state::get().move_up(edit_index);
                    queue_edit_mode = Some(new_index);
                }
                Some(KeyboardKey::KEY_ONE) => queue_edit_mode = Some(1),
                Some(KeyboardKey::KEY_TWO) => queue_edit_mode = Some(2),
                Some(KeyboardKey::KEY_THREE) => queue_edit_mode = Some(3),
                Some(KeyboardKey::KEY_FOUR) => queue_edit_mode = Some(4),
                Some(KeyboardKey::KEY_FIVE) => queue_edit_mode = Some(5),
                Some(KeyboardKey::KEY_SIX) => queue_edit_mode = Some(6),
                Some(KeyboardKey::KEY_SEVEN) => queue_edit_mode = Some(7),
                Some(KeyboardKey::KEY_EIGHT) => queue_edit_mode = Some(8),
                Some(KeyboardKey::KEY_NINE) => queue_edit_mode = Some(9),
                _ => (),
            }
        } else {
            match rl.get_key_pressed() {
                Some(KeyboardKey::KEY_N) => event_tx.send(Event::NextSong).unwrap(),
                Some(KeyboardKey::KEY_SPACE) => event_tx.send(Event::TogglePause).unwrap(),
                Some(KeyboardKey::KEY_Q) => qr_contrast = qr_contrast.saturating_sub(10),
                Some(KeyboardKey::KEY_W) => qr_contrast = qr_contrast.saturating_add(10),
                Some(KeyboardKey::KEY_A) => qr_size = qr_size.saturating_sub(1).max(1),
                Some(KeyboardKey::KEY_S) => qr_size = qr_size.saturating_add(1),
                Some(KeyboardKey::KEY_ONE) => queue_edit_mode = Some(1),
                Some(KeyboardKey::KEY_TWO) => queue_edit_mode = Some(2),
                Some(KeyboardKey::KEY_THREE) => queue_edit_mode = Some(3),
                Some(KeyboardKey::KEY_FOUR) => queue_edit_mode = Some(4),
                Some(KeyboardKey::KEY_FIVE) => queue_edit_mode = Some(5),
                Some(KeyboardKey::KEY_SIX) => queue_edit_mode = Some(6),
                Some(KeyboardKey::KEY_SEVEN) => queue_edit_mode = Some(7),
                Some(KeyboardKey::KEY_EIGHT) => queue_edit_mode = Some(8),
                Some(KeyboardKey::KEY_NINE) => queue_edit_mode = Some(9),
                _ => (),
            }
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
        let mut song_index = 0;

        if !state.has_song_suggestions() {
            d.draw_text_ex(
                &font_bold,
                "No song suggestions queued!",
                rvec2(100, y),
                FONT_SIZE_BOLD as f32,
                0.0,
                Color::DIMGRAY,
            );
            y += 24.0;
            d.draw_text_ex(
                &font_bold,
                "Scan the QR code to suggest a song.",
                rvec2(100, y),
                FONT_SIZE_BOLD as f32,
                0.0,
                Color::DIMGRAY,
            );
            y += 32.0;
        }

        for song in &mut queue {
            song_index += 1;

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

            if let Some(edit_index) = queue_edit_mode
                && edit_index == song_index
            {
                d.draw_text_ex(
                    &font_bold,
                    ">",
                    rvec2(70, y + 21.0),
                    FONT_SIZE_BOLD as f32,
                    0.0,
                    Color::new(50, 50, 50, 255),
                );
            }

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
        } else if state.has_fallback_queue() && y as i32 <= screen_height - 220 {
            /* fallback queue *********************************************************************/

            let msg = match state.has_song_suggestions() {
                true => "Fallback Queue (will be played when suggestions run out):",
                false => "Fallback Queue:",
            };

            y += 32.0;
            d.draw_text_ex(
                &font_bold,
                msg,
                rvec2(100, y),
                FONT_SIZE_BOLD as f32,
                0.0,
                Color::DIMGRAY,
            );
            y += 36.0;

            let mut fallback_queue = state.fallback_queue();

            for song in &mut fallback_queue {
                song_index += 1;

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

                if let Some(edit_index) = queue_edit_mode
                    && edit_index == song_index
                {
                    d.draw_text_ex(
                        &font_bold,
                        ">",
                        rvec2(70, y + 21.0),
                        FONT_SIZE_BOLD as f32,
                        0.0,
                        Color::new(50, 50, 50, 255),
                    );
                }

                y += 80.0;

                if y as i32 > screen_height - 160 {
                    break;
                }
            }

            let remaining = fallback_queue.count();
            if remaining > 0 {
                let plural = if remaining == 1 { "" } else { "s" };
                d.draw_text_ex(
                    &font_bold,
                    &format!("{remaining} more song{plural} in fallback queue"),
                    rvec2(100, y),
                    FONT_SIZE_BOLD as f32,
                    0.0,
                    Color::DIMGRAY,
                );
            }
        }

        if let Some(edit_index) = queue_edit_mode
            && edit_index > song_index
        {
            queue_edit_mode = None;
        }

        /* connection status **********************************************************************/

        match state.connection_state() {
            ConnectionState::NotConnected => {
                let msg = "not connected";
                let text_width = font_bold.measure_text(msg, FONT_SIZE_BOLD as f32, 0.0).x as i32;
                let x = screen_width - text_width - 20;
                let y = screen_height - FONT_SIZE_BOLD - 20;
                d.draw_text_ex(
                    &font_bold,
                    msg,
                    rvec2(x, y),
                    FONT_SIZE_BOLD as f32,
                    0.0,
                    Color::MAROON,
                );
            }
            ConnectionState::Connected { id } => {
                let url = util::submission_url(id, &server_address, server_port);

                let qr_color = Color::new(qr_contrast, qr_contrast, qr_contrast, 255);

                let text_width = font_bold.measure_text(&url, FONT_SIZE_BOLD as f32, 0.0).x as i32;
                let x = screen_width - text_width - 20;
                let y = screen_height - FONT_SIZE_BOLD - 20;
                d.draw_text_ex(
                    &font_bold,
                    &url,
                    rvec2(x, y),
                    FONT_SIZE_BOLD as f32,
                    0.0,
                    qr_color,
                );

                let qr = server_qrcode.as_ref().unwrap();
                let size = 29 * (qr_size as i32);
                let x = screen_width - size - 20;
                let y = screen_height - size - 55;
                d.draw_texture_pro(
                    qr,
                    rrect(0, 0, qr.width(), qr.height()),
                    rrect(x, y, size, size),
                    rvec2(0, 0),
                    0.0,
                    qr_color,
                );
            }
            ConnectionState::Error { msg } => {
                let msg = format!("error: {msg}");
                let text_width = font_bold.measure_text(&msg, FONT_SIZE_BOLD as f32, 0.0).x as i32;
                let x = screen_width - text_width - 20;
                let y = screen_height - FONT_SIZE_BOLD - 20;
                d.draw_text_ex(
                    &font_bold,
                    &msg,
                    rvec2(x, y),
                    FONT_SIZE_BOLD as f32,
                    0.0,
                    Color::MAROON,
                );
            }
        };

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
        let state = state::get();
        for song in state
            .playing()
            .map(|playing| &playing.song)
            .into_iter()
            .chain(state.queue())
            .chain(state.fallback_queue())
        {
            if let Entry::Vacant(entry) = self.thumbnails.entry(song.id.to_owned()) {
                let image = Image::load_image_from_mem(".png", &song.thumbnail).unwrap();
                let mut texture = rl.load_texture_from_image(thread, &image).unwrap();
                texture.gen_texture_mipmaps();
                texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
                entry.insert(texture);
            }
        }
    }

    fn default_texture(rl: &mut RaylibHandle, thread: &RaylibThread) -> Texture2D {
        let image = Image::gen_image_color(48, 48, Color::new(20, 20, 20, 255));
        rl.load_texture_from_image(thread, &image).unwrap()
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

fn generate_qr_texture(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    texture_out: &mut Option<Texture2D>,
    url: &str,
) {
    let qrcode = QrCode::new(url.as_bytes()).unwrap();
    let image = qrcode
        .render::<Luma<u8>>()
        .dark_color(Luma([255]))
        .light_color(Luma([0]))
        .quiet_zone(false)
        .build();
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    image.write_to(&mut cursor, ImageFormat::Png).unwrap();
    let image = Image::load_image_from_mem(".png", &buffer).unwrap();
    let texture = rl.load_texture_from_image(thread, &image).unwrap();
    *texture_out = Some(texture);
}
