//! Daily log files written by the GUI: `dpimech-YYYY-MM-DD.log` in a folder the user picks
//! (off by default), and `dpimech-error-….log` snapshots of the recent log when a profile
//! stops with an error (always, so a failure can be explained afterwards).
//!
//! The service runs as SYSTEM/root, so it must never write to a path a client chose (that
//! would be an arbitrary privileged file write). The GUI already receives every log line and
//! runs as the user, so it writes the files with the user's own rights.

use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use dpimech_core::model::{LogLevel, LogLine};

/// Files older than this many days are deleted when logging starts.
const KEEP_DAYS: i64 = 14;
const PREFIX: &str = "dpimech-";

/// Error snapshots kept in the folder; older ones are deleted.
const KEEP_SNAPSHOTS: usize = 10;
const SNAPSHOT_PREFIX: &str = "dpimech-error-";
/// Lines kept in memory for error snapshots.
const RECENT_LINES: usize = 500;

/// `None` while daily files are turned off.
static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
/// Where error snapshots go (the chosen log folder, or the default one).
static SNAPSHOT_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

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

/// Turns daily files on (with the folder) or off; `snapshots` is where error snapshots go.
pub fn configure(dir: Option<PathBuf>, snapshots: Option<PathBuf>) {
    if let Some(dir) = &dir {
        let _ = std::fs::create_dir_all(dir);
        prune(dir);
    }
    *DIR.lock().unwrap() = dir;
    *SNAPSHOT_DIR.lock().unwrap() = snapshots;
}

/// Keeps lines the GUI got in one go (the service's recent log on connect) for error
/// snapshots; they were written to the daily file when they first happened, if at all.
pub fn remember(lines: &[LogLine]) {
    let mut recent = RECENT.lock().unwrap();
    recent.clear();
    let skip = lines.len().saturating_sub(RECENT_LINES);
    recent.extend(lines.iter().skip(skip).filter_map(format_line));
}

fn format_line(line: &LogLine) -> Option<String> {
    let time = chrono::DateTime::from_timestamp_millis(line.unix_ms as i64)?;
    let local = time.with_timezone(&chrono::Local);
    let level = match line.level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO ",
        LogLevel::Warn => "WARN ",
        LogLevel::Error => "ERROR",
    };
    Some(format!(
        "{} {level} [{}] {}\n",
        local.format("%H:%M:%S"),
        line.source,
        line.text
    ))
}

pub fn write(line: &LogLine) {
    let Some(time) = chrono::DateTime::from_timestamp_millis(line.unix_ms as i64) else {
        return;
    };
    let local = time.with_timezone(&chrono::Local);
    let Some(text) = format_line(line) else {
        return;
    };
    {
        let mut recent = RECENT.lock().unwrap();
        if recent.len() == RECENT_LINES {
            recent.pop_front();
        }
        recent.push_back(text.clone());
    }
    let Some(dir) = DIR.lock().unwrap().clone() else {
        return;
    };
    let path = dir.join(format!("{PREFIX}{}.log", local.format("%Y-%m-%d")));
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(text.as_bytes());
    }
}

/// Writes the recent log to `dpimech-error-<date>_<time>.log` after `profile` failed.
pub fn save_error_snapshot(profile: &str) -> Option<PathBuf> {
    let dir = SNAPSHOT_DIR.lock().unwrap().clone()?;
    std::fs::create_dir_all(&dir).ok()?;
    let now = chrono::Local::now();
    let path = dir.join(format!(
        "{SNAPSHOT_PREFIX}{}.log",
        now.format("%Y-%m-%d_%H-%M-%S")
    ));
    let mut text = format!(
        "DPIMech {} — \"{profile}\" stopped with an error at {}\n\n",
        dpimech_core::VERSION,
        now.format("%Y-%m-%d %H:%M:%S")
    );
    text.extend(RECENT.lock().unwrap().iter().cloned());
    std::fs::write(&path, text).ok()?;
    prune_snapshots(&dir);
    Some(path)
}

/// Keeps the newest error snapshots (their names sort by time).
fn prune_snapshots(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut snapshots: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(SNAPSHOT_PREFIX) && n.ends_with(".log"))
        })
        .collect();
    snapshots.sort();
    let excess = snapshots.len().saturating_sub(KEEP_SNAPSHOTS);
    for old in &snapshots[..excess] {
        let _ = std::fs::remove_file(old);
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

        configure(Some(dir.clone()), Some(dir.clone()));
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
        let snapshot = save_error_snapshot("Discord").unwrap();
        configure(None, None);
        assert!(
            std::fs::read_to_string(&snapshot)
                .unwrap()
                .contains("[Discord] connection slow")
        );

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
