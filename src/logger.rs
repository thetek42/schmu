use log::{Level, LevelFilter, Log, Metadata, Record};

pub fn init() {
    init_with_level(LevelFilter::Info)
}

pub fn init_with_level(level: LevelFilter) {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(level);
}

static LOGGER: Logger = Logger {};

struct Logger {}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        _ = metadata;
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_str = match record.level() {
                Level::Error => "\x1b[31mERROR\x1b[0m",
                Level::Warn => "\x1b[33mWARN \x1b[0m",
                Level::Info => "\x1b[32mINFO \x1b[0m",
                Level::Debug => "\x1b[34mDEBUG\x1b[0m",
                Level::Trace => "\x1b[35mTRACE\x1b[0m",
            };

            println!(
                "{level_str} \x1b[90m{}:\x1b[0m {}",
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
