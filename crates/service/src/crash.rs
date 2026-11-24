//! Crash reports for the service. A panic writes what happened to `<data>/logs/crash.txt`. The
//! service is restarted by the system (Windows service recovery, systemd `Restart=on-failure`),
//! and a panic inside one task leaves the service running with that part broken; either way the
//! next app that connects gets the text in its `Hello` reply and offers to report it.

use std::path::PathBuf;
use std::sync::OnceLock;

use dpimech_core::crashtext;

static PATH: OnceLock<PathBuf> = OnceLock::new();

/// Writes every panic to the crash file, then lets the default hook print it as before.
pub fn install(logs_dir: PathBuf) {
    if PATH.set(logs_dir.join("crash.txt")).is_err() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = PATH.get() {
            let text = crashtext::describe(
                "dpimech-service",
                &format!("unix time {}", crate::logs::now_ms() / 1000),
                std::thread::current().name().unwrap_or("unnamed"),
                &info.to_string(),
                &std::backtrace::Backtrace::force_capture().to_string(),
            );
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let old = std::fs::read_to_string(path).unwrap_or_default();
            let _ = std::fs::write(path, crashtext::append(&old, &text));
        }
        previous(info);
    }));
}

/// The recorded crash, handed to the first app that asks; reported only once.
pub fn take() -> Option<String> {
    let path = PATH.get()?;
    let text = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}
