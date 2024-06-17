use std::{io::Write, sync::{Mutex, OnceLock}};

use log::*;
use colored::Colorize;

static LV: LevelFilter = LevelFilter::Debug;
const LOG_TO_STD_IN: bool = true;

pub struct StaticLogger(Mutex<std::io::Stderr>);
impl Log for StaticLogger {
    fn flush(&self) {
        if LOG_TO_STD_IN { return; }
        loop {
            self.0.clear_poison();
            if let Ok(_) = self.0.try_lock().map(|mut lock| {
                lock.flush()
            }) { break }
        }
    }
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
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
        if LOG_TO_STD_IN {
            return println!("{mark:<8}{file}:{:<n$} {}", record.line().unwrap_or(0), record.args())
        }
        loop {
            self.0.clear_poison();
            if let Ok(_) = self.0.try_lock().map(|mut lock| {
                writeln!(lock, "{mark:<8}{file}:{:<n$} {}", record.line().unwrap_or(0), record.args()).unwrap()
            }) { break }
        }
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    static STATICLOGGER: OnceLock<StaticLogger> = OnceLock::new();
    log::set_logger(STATICLOGGER.get_or_init(|| StaticLogger(Mutex::new(std::io::stderr()))))
        .map(|()| log::set_max_level(LV))
}