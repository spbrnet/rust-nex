use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG: OnceLock<Option<Mutex<File>>> = OnceLock::new();

fn log() -> Option<&'static Mutex<File>> {
    LOG.get_or_init(|| {
        let path = std::env::var_os("RNEX_DIAGNOSTIC_LOG")?;
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = match options.open(&path) {
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
        let line = format!(
            "time_unix_ns={timestamp} process={} event={event} {detail}\n",
            std::process::id()
        );
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn record_bytes(event: &str, detail: impl std::fmt::Display, data: &[u8]) {
    if !enabled() {
        return;
    }
    record(
        event,
        format_args!("{detail} bytes={} hex={}", data.len(), hex::encode(data)),
    );
}
