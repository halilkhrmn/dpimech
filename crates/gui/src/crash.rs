//! Crash reports for the app. A panic (in any thread) writes what happened to `crash.txt` in the
//! config folder; the next start finds it and opens "Report a problem" with it. Only panics are
//! caught: an app killed by shutdown or sign-out leaves nothing, so it is never mistaken for a
//! crash. The service keeps its own file and hands it over in `Hello` (see the service's
//! `crash.rs`).

use std::path::PathBuf;

use dpimech_core::crashtext;

fn path() -> Option<PathBuf> {
    crate::prefs::config_file("crash.txt")
}

/// Writes every panic to the crash file, then lets the default hook print it as before.
pub fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = path() {
            let text = crashtext::describe(
                "DPIMech",
                &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                std::thread::current().name().unwrap_or("unnamed"),
                &info.to_string(),
                &std::backtrace::Backtrace::force_capture().to_string(),
            );
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let old = std::fs::read_to_string(&path).unwrap_or_default();
            let _ = std::fs::write(&path, crashtext::append(&old, &text));
        }
        previous(info);
    }));
}

/// The crash left by the previous run, if any; it is only reported once.
pub fn take_last() -> Option<String> {
    let path = path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}
