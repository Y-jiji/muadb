use std::{collections::HashMap, io::Write, sync::{atomic::AtomicBool, Mutex, OnceLock}};

use log::*;
use Level::*;
use colored::Colorize;

static LV: LevelFilter = LevelFilter::Debug;

pub struct StaticLogger(Mutex<std::io::Stderr>);
impl Log for StaticLogger {
    fn flush(&self) {
        self.0.clear_poison();
        self.0.try_lock().map(|mut lock| lock.flush().unwrap());
    }
    fn enabled(&self, metadata: &Metadata) -> bool {
        return true;
    }
    fn log(&self, record: &Record) {
        use std::io::Write;
        if !self.enabled(record.metadata()) { return }
        let mark = match record.level() {
            Level::Debug => "{DEBUG}".bright_black(),
            Level::Trace => "{TRACE}".bright_black(),
            Level::Info  => "{INFO}".cyan(),
            Level::Warn  => "{WARN}".yellow(),
            Level::Error => "{ERROR}".red()
        }.bold();
        let file = record.file_static().unwrap_or("");
        let n = 30-file.len();
        self.0.lock().map(|mut lock| {
            writeln!(lock, "{mark:<8}{file}:{:<n$} {}", record.line().unwrap_or(0), record.args()).unwrap()
        });
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    static STATICLOGGER: OnceLock<StaticLogger> = OnceLock::new();
    log::set_logger(STATICLOGGER.get_or_init(|| StaticLogger(Mutex::new(std::io::stderr()))))
        .map(|()| log::set_max_level(LV.into()))
}