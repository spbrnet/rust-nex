use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG: OnceLock<Option<Mutex<File>>> = OnceLock::new();

fn log() -> Option<&'static Mutex<File>> {
    LOG.get_or_init(|| {
        let path = std::env::var_os("RNEX_DIAGNOSTIC_LOG")?;
        let file = match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => file,
            Err(error) => {
                eprintln!("failed to open RNEX_DIAGNOSTIC_LOG {path:?}: {error}");
                return None;
            }
        };
        Some(Mutex::new(file))
    })
    .as_ref()
}

pub fn enabled() -> bool {
    log().is_some()
}

pub fn record(event: &str, detail: impl std::fmt::Display) {
    let Some(log) = log() else {
        return;
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    if let Ok(mut file) = log.lock() {
        let _ = writeln!(file, "time_unix_ns={timestamp} process={} event={event} {detail}", std::process::id());
    }
}

pub fn record_bytes(event: &str, detail: impl std::fmt::Display, data: &[u8]) {
    if !enabled() {
        return;
    }
    record(event, format_args!("{detail} bytes={} hex={}", data.len(), hex::encode(data)));
}
