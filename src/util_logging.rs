use std::{collections::HashMap, sync::{atomic::AtomicBool, OnceLock}};

use log::*;
use Level::*;
use colored::Colorize;

static LV: LevelFilter = LevelFilter::Debug;
static LOGGER: StaticLogger = StaticLogger;

pub struct StaticLogger;
impl Log for StaticLogger {
    fn flush(&self) {
        println!("flush");
    }
    fn enabled(&self, metadata: &Metadata) -> bool {
        return true;
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) { return }
        let mark = match record.level() {
            Level::Debug => "{DEBUG}".bright_black(),
            Level::Trace => "{TRACE}".bright_black(),
            Level::Info  => "{INFO}".cyan(),
            Level::Warn  => "{WARN}".yellow(),
            Level::Error => "{ERROR}".red()
        }.bold();
        println!("{mark:<10}{:>30} {:>4} {}", record.module_path().unwrap_or(""), record.line().unwrap_or(0), record.args())
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    if INITIALIZED.fetch_or(true, std::sync::atomic::Ordering::SeqCst) {
        return Ok(())
    }
    log::set_logger(&StaticLogger)
        .map(|()| log::set_max_level(LV.into()))
}