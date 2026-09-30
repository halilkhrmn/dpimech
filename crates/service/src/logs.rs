use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use dpimech_core::ipc::Event;
use dpimech_core::model::{LogLevel, LogLine};
use tokio::sync::broadcast;

const CAPACITY: usize = 1000;

/// Ring buffer of user-visible log lines; every push is also broadcast to connected GUIs.
pub struct LogBus {
    lines: Mutex<VecDeque<LogLine>>,
    events: broadcast::Sender<Event>,
}

impl LogBus {
    pub fn new(events: broadcast::Sender<Event>) -> Self {
        Self {
            lines: Mutex::new(VecDeque::with_capacity(CAPACITY)),
            events,
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
