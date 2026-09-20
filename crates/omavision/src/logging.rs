//! Logs to stderr and appends to `<state_dir>/omavision.log`. Level from `RUST_LOG`.
use log::{LevelFilter, Log, Metadata, Record};
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

struct Logger {
    file: Option<Mutex<File>>,
}

impl Log for Logger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= log::max_level()
    }

    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let line = format!("{} {:<5} {}", omavision_core::now(), r.level(), r.args());
        eprintln!("{line}");
        if let Some(f) = &self.file {
            if let Ok(mut f) = f.lock() {
                let _ = writeln!(f, "{line}");
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    let level = std::env::var("RUST_LOG").ok().and_then(|s| s.trim().parse().ok()).unwrap_or(LevelFilter::Info);
    let dir = omavision_core::config::state_dir();
    let file = std::fs::create_dir_all(&dir)
        .ok()
        .and_then(|_| std::fs::OpenOptions::new().create(true).append(true).open(dir.join("omavision.log")).ok())
        .map(Mutex::new);
    log::set_max_level(level);
    let _ = log::set_boxed_logger(Box::new(Logger { file }));
}
