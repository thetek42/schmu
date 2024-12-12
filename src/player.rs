use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use libmpv2::events::{Event, EventContext};
use libmpv2::Mpv;

use crate::{state, util};

pub struct Player {
    tx: Sender<Message>,
}

impl Player {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();

        log::info!("starting player");
        _ = thread::spawn(move || PlayerThread::run(rx));

        Self { tx }
    }

    pub fn next(&self) {
        let msg = Message::Next;
        self.tx.send(msg).unwrap();
    }

    fn quit(&self) {
        log::info!("terminating player");
        let msg = Message::Quit;
        self.tx.send(msg).unwrap();
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.quit();
    }
}

enum Message {
    Quit,
    Next,
}

struct PlayerThread {
    rx: Receiver<Message>,
}

impl PlayerThread {
    fn run(rx: Receiver<Message>) {
        let player = Self { rx };

        while player.run_iter() {}
    }

    fn run_iter(&self) -> bool {
        match self.rx.try_recv() {
            Ok(Message::Quit) => return false,
            Ok(Message::Next) => (),
            Err(TryRecvError::Disconnected) => return false,
            Err(TryRecvError::Empty) => (),
        }

        let Some(next_song_id) = self.get_next_song() else {
            thread::sleep(Duration::from_millis(50));
            return true;
        };

        match self.play(&next_song_id) {
            Ok(quit) => quit,
            Err(e) => {
                log::error!("failed to play {next_song_id}: {e}");
                true
            }
        }
    }

    fn get_next_song(&self) -> Option<String> {
        let mut state = state::get();
        state.get_next_song()
    }

    fn play(&self, id: &str) -> Result<bool, libmpv2::Error> {
        log::info!("playing {id}");

        let path = util::audio_cache_location(id);
        let path = path.to_str().unwrap();

        let mpv = Mpv::new()?;
        mpv.command("loadfile", &[path, "replace"])?;

        let mut event = EventContext::new(mpv.ctx);

        loop {
            match self.rx.try_recv() {
                Ok(Message::Quit) => return Ok(false),
                Ok(Message::Next) => return Ok(true),
                Err(TryRecvError::Disconnected) => return Ok(false),
                Err(TryRecvError::Empty) => (),
            }

            if let Ok((total, elapsed)) = get_time(&mpv) {
                let mut state = state::get();
                state.update_current_time(total, elapsed);
            }

            match event.wait_event(0.05) {
                Some(Ok(Event::Shutdown)) => {
                    log::info!("done playing {id}");
                    return Ok(true);
                }
                Some(Ok(Event::EndFile(r))) => {
                    log::info!("mpv reached endfile {id} with reason {r}");
                    return Ok(true);
                }
                Some(Ok(_)) => (),
                Some(Err(e)) => {
                    log::warn!("mpv got error: {e}");
                    return Ok(true);
                }
                None => (),
            }
        }
    }
}

fn get_time(mpv: &Mpv) -> Result<(Duration, Duration), libmpv2::Error> {
    let total_secs: i64 = mpv.get_property("duration")?;
    let elapsed_secs: i64 = mpv.get_property("time-pos")?;
    let total = Duration::from_secs(total_secs as u64);
    let elapsed = Duration::from_secs(elapsed_secs as u64);
    Ok((total, elapsed))
}
