//! Daily log files written by the GUI: `dpimech-YYYY-MM-DD.log` in a folder the user picks.
//!
//! The service runs as SYSTEM/root, so it must never write to a path a client chose (that
//! would be an arbitrary privileged file write). The GUI already receives every log line and
//! runs as the user, so it writes the files with the user's own rights.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use dpimech_core::model::{LogLevel, LogLine};

/// Files older than this many days are deleted when logging starts.
const KEEP_DAYS: i64 = 14;
const PREFIX: &str = "dpimech-";

/// `None` while writing is turned off.
static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// `%LOCALAPPDATA%\DPIMech\logs`, or `$XDG_STATE_HOME/dpimech/logs` (`~/.local/state/...`).
pub fn default_dir() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        return Some(PathBuf::from(base).join("DPIMech").join("logs"));
    }
    if cfg!(target_os = "macos") {
        return std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Logs/DPIMech"));
    }
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .map(|base| base.join("dpimech").join("logs"))
}

/// Turns writing on (with the folder) or off.
pub fn configure(dir: Option<PathBuf>) {
    if let Some(dir) = &dir {
        let _ = std::fs::create_dir_all(dir);
        prune(dir);
    }
    *DIR.lock().unwrap() = dir;
}

pub fn write(line: &LogLine) {
    let Some(dir) = DIR.lock().unwrap().clone() else {
        return;
    };
    let Some(time) = chrono::DateTime::from_timestamp_millis(line.unix_ms as i64) else {
        return;
    };
    let local = time.with_timezone(&chrono::Local);
    let path = dir.join(format!("{PREFIX}{}.log", local.format("%Y-%m-%d")));
    let level = match line.level {
        LogLevel::Info => "INFO ",
        LogLevel::Warn => "WARN ",
        LogLevel::Error => "ERROR",
    };
    let text = format!(
        "{} {level} [{}] {}\n",
        local.format("%H:%M:%S"),
        line.source,
        line.text
    );
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(text.as_bytes());
    }
}

/// Deletes our own dated files past the retention window; nothing else in the folder.
fn prune(dir: &Path) {
    let cutoff = chrono::Local::now().date_naive() - chrono::Days::new(KEEP_DAYS as u64);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let date = name
            .strip_prefix(PREFIX)
            .and_then(|rest| rest.strip_suffix(".log"))
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
        if date.is_some_and(|d| d < cutoff) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_daily_files_and_prunes_only_old_ones() {
        let dir = std::env::temp_dir().join(format!("dpimech-logfile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("dpimech-2000-01-01.log"), "old").unwrap();
        std::fs::write(dir.join("notes.txt"), "keep").unwrap();

        configure(Some(dir.clone()));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        write(&LogLine {
            unix_ms: now,
            level: LogLevel::Warn,
            source: "Discord".into(),
            text: "connection slow".into(),
        });
        configure(None);

        assert!(!dir.join("dpimech-2000-01-01.log").exists());
        assert!(dir.join("notes.txt").exists());
        let today = chrono::Local::now().format("%Y-%m-%d");
        let text = std::fs::read_to_string(dir.join(format!("dpimech-{today}.log"))).unwrap();
        assert!(
            text.ends_with("WARN  [Discord] connection slow\n"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
