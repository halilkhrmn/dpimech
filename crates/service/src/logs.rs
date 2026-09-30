use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dpimech_core::ipc::Event;
use dpimech_core::model::{LogLevel, LogLine};
use tokio::sync::broadcast;

const CAPACITY: usize = 1000;

/// Ring buffer of user-visible log lines; every push is also broadcast to connected GUIs.
pub struct LogBus {
    lines: Mutex<VecDeque<LogLine>>,
    events: broadcast::Sender<Event>,
    /// Detailed lines (per connection, per strategy) are kept only while someone asked for them.
    detailed: AtomicBool,
}

impl LogBus {
    pub fn new(events: broadcast::Sender<Event>) -> Self {
        Self {
            lines: Mutex::new(VecDeque::with_capacity(CAPACITY)),
            events,
            detailed: AtomicBool::new(false),
        }
    }

    pub fn push(&self, level: LogLevel, source: &str, text: impl Into<String>) {
        let line = LogLine {
            unix_ms: now_ms(),
            level,
            source: source.to_owned(),
            text: text.into(),
        };
        match level {
            LogLevel::Debug => tracing::debug!(source = %line.source, "{}", line.text),
            LogLevel::Info => tracing::info!(source = %line.source, "{}", line.text),
            LogLevel::Warn => tracing::warn!(source = %line.source, "{}", line.text),
            LogLevel::Error => tracing::error!(source = %line.source, "{}", line.text),
        }
        {
            let mut lines = self.lines.lock().unwrap();
            if lines.len() == CAPACITY {
                lines.pop_front();
            }
            lines.push_back(line.clone());
        }
        let _ = self.events.send(Event::Log { line });
    }

    pub fn set_detailed(&self, on: bool) {
        self.detailed.store(on, Ordering::Relaxed);
    }

    pub fn detailed(&self) -> bool {
        self.detailed.load(Ordering::Relaxed)
    }

    /// A troubleshooting line; dropped unless the detailed log is on. `text` is only built
    /// when it will be used.
    pub fn debug(&self, source: &str, text: impl FnOnce() -> String) {
        if self.detailed() {
            self.push(LogLevel::Debug, source, text());
        }
    }

    pub fn warn(&self, source: &str, text: impl Into<String>) {
        self.push(LogLevel::Warn, source, text);
    }

    pub fn info(&self, source: &str, text: impl Into<String>) {
        self.push(LogLevel::Info, source, text);
    }

    pub fn error(&self, source: &str, text: impl Into<String>) {
        self.push(LogLevel::Error, source, text);
    }

    pub fn recent(&self) -> Vec<LogLine> {
        self.lines.lock().unwrap().iter().cloned().collect()
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
