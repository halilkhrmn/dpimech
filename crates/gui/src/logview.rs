//! The Logs page: every recent line is kept here, the page shows the ones that match the filter.

use std::cell::RefCell;

use slint::{Model, ModelRc, VecModel};

use crate::{AppWindow, LogItem};

pub const MAX_ROWS: usize = 1000;

thread_local! {
    /// Newest first, like the page.
    static ALL: RefCell<Vec<LogItem>> = const { RefCell::new(Vec::new()) };
    /// Lower-cased words that must all occur in a line.
    static FILTER: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn matches(item: &LogItem, words: &[String]) -> bool {
    if words.is_empty() {
        return true;
    }
    let line = format!("{} {} {}", item.source, item.level, item.text).to_lowercase();
    words.iter().all(|w| line.contains(w))
}

fn show(ui: &AppWindow) {
    let rows: Vec<LogItem> = FILTER.with_borrow(|words| {
        ALL.with_borrow(|all| all.iter().filter(|i| matches(i, words)).cloned().collect())
    });
    ui.set_logs(ModelRc::new(VecModel::from(rows)));
}

/// Replaces everything; `items` newest first.
pub fn replace(ui: &AppWindow, mut items: Vec<LogItem>) {
    items.truncate(MAX_ROWS);
    ALL.set(items);
    show(ui);
}

pub fn append(ui: &AppWindow, item: LogItem) {
    let visible = FILTER.with_borrow(|words| matches(&item, words));
    ALL.with_borrow_mut(|all| {
        all.insert(0, item.clone());
        all.truncate(MAX_ROWS);
    });
    if !visible {
        return;
    }
    let logs = ui.get_logs();
    if let Some(model) = logs.as_any().downcast_ref::<VecModel<LogItem>>() {
        model.insert(0, item);
        if model.row_count() > MAX_ROWS {
            model.remove(MAX_ROWS);
        }
    }
}

pub fn clear(ui: &AppWindow) {
    ALL.set(Vec::new());
    show(ui);
}

pub fn set_filter(ui: &AppWindow, text: &str) {
    FILTER.set(text.split_whitespace().map(str::to_lowercase).collect());
    show(ui);
}

/// Every recent line, newest first, whatever the filter says (for "Report a problem").
pub fn all() -> Vec<LogItem> {
    ALL.with_borrow(|all| all.clone())
}

#[cfg(test)]
mod tests {
    use super::matches;
    use crate::LogItem;

    #[test]
    fn every_word_must_occur_in_source_level_or_text() {
        let item = LogItem {
            time: "12:00:00".into(),
            source: "Discord".into(),
            text: "connection slow; restarting the engine".into(),
            level: "warn".into(),
        };
        let words =
            |s: &str| -> Vec<String> { s.split_whitespace().map(str::to_lowercase).collect() };
        assert!(matches(&item, &words("")));
        assert!(matches(&item, &words("discord")));
        assert!(matches(&item, &words("warn slow")));
        assert!(!matches(&item, &words("discord error")));
    }
}
