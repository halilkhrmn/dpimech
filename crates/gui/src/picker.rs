//! Application picker for per-app routing.

use crate::i18n::tr;
use slint::{ComponentHandle, Image, Model, ModelRc, VecModel, Weak};

use crate::apps::{self, exe_stem};
use crate::convert::app_item;
use crate::state::{DRAFT_APPS, ICONS, PICKER_ALL, PICKER_QUERY};

thread_local! {
    /// An app-proxy profile opens exactly one app: picking another replaces it.
    static SINGLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn set_single(single: bool) {
    SINGLE.set(single);
}
use crate::{AppItem, AppWindow};

/// Rediscovers apps on a worker thread; icon extraction can take a moment.
pub fn refresh(ui: Weak<AppWindow>) {
    std::thread::spawn(move || {
        let found = apps::discover();
        let _ = ui.upgrade_in_event_loop(move |ui| {
            let items: Vec<AppItem> = found
                .into_iter()
                .map(|a| {
                    let icon = a.icon.map(Image::from_rgba8);
                    if let Some(icon) = &icon {
                        ICONS.with_borrow_mut(|m| {
                            m.insert(a.exe.to_lowercase(), (a.title.clone(), icon.clone()))
                        });
                    }
                    app_item(&a.title, &a.exe, &a.path, a.running, icon)
                })
                .collect();
            PICKER_ALL.set(items);
            apply_filter(&ui);
            ui.set_picker_loading(false);
            refresh_draft_icons();
        });
    });
}

pub fn open(ui: &AppWindow) {
    PICKER_QUERY.set(String::new());
    let empty = PICKER_ALL.with_borrow(|a| a.is_empty());
    ui.set_picker_loading(empty);
    apply_filter(ui);
    ui.set_picker_open(true);
    refresh(ui.as_weak());
}

pub fn search(ui: &AppWindow, query: &str) {
    PICKER_QUERY.set(query.trim().to_lowercase());
    apply_filter(ui);
}

fn apply_filter(ui: &AppWindow) {
    let query = PICKER_QUERY.with_borrow(|q| q.clone());
    let items: Vec<AppItem> = PICKER_ALL.with_borrow(|all| {
        all.iter()
            .filter(|a| {
                query.is_empty()
                    || a.title.to_lowercase().contains(&query)
                    || a.exe.to_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    });
    ui.set_picker_apps(ModelRc::new(VecModel::from(items)));
}

pub fn pick(ui: &AppWindow, app: AppItem) {
    add_to_draft(app);
    ui.set_picker_open(false);
}

/// A bare name matches the executable name; anything with a slash matches part of the path.
pub fn add_manual(ui: &AppWindow, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let key = if text.contains(['\\', '/']) {
        text.to_owned()
    } else {
        exe_stem(text)
    };
    add_to_draft(item_for_key(&key));
    ui.set_picker_open(false);
}

/// Adds an app by name without the dialog (used by the setup wizard).
pub fn add_manual_key(key: &str) {
    add_to_draft(item_for_key(key));
}

pub fn browse(ui: &AppWindow) {
    let Some(path) = rfd::FileDialog::new()
        .set_title(tr("Choose an application"))
        .add_filter(tr("Applications"), &["exe"])
        .pick_file()
    else {
        return;
    };
    let path = path.to_string_lossy().into_owned();
    let exe = exe_stem(&path);
    let icon = apps::icon_for_path(&path).map(Image::from_rgba8);
    if let Some(icon) = &icon {
        ICONS.with_borrow_mut(|m| m.insert(exe.to_lowercase(), (exe.clone(), icon.clone())));
    }
    add_to_draft(app_item(&exe, &exe, &path, false, icon));
    ui.set_picker_open(false);
}

pub fn remove(index: i32) {
    DRAFT_APPS.with(|m| {
        if let Ok(i) = usize::try_from(index)
            && i < m.row_count()
        {
            m.remove(i);
        }
    });
}

/// Replaces the editor's app list with saved routing keys.
pub fn load_draft(keys: &[String]) {
    let items: Vec<AppItem> = keys.iter().map(|k| item_for_key(k)).collect();
    DRAFT_APPS.with(|m| m.set_vec(items));
}

/// The app of an app-proxy profile, by its full path.
pub fn load_draft_app(path: &str) {
    let exe = exe_stem(path);
    let icon = apps::icon_for_path(path).map(Image::from_rgba8);
    let title = ICONS
        .with_borrow(|m| m.get(&exe.to_lowercase()).map(|(t, _)| t.clone()))
        .unwrap_or_else(|| crate::launcher::display_name(path));
    DRAFT_APPS.with(|m| m.set_vec(vec![app_item(&title, &exe, path, false, icon)]));
}

/// What an app-proxy profile should open: the chosen app's path, else the typed name.
pub fn draft_app() -> Option<String> {
    DRAFT_APPS.with(|m| {
        m.row_data(0).map(|a| {
            if a.path.is_empty() {
                a.exe.to_string()
            } else {
                a.path.to_string()
            }
        })
    })
}

pub fn draft_keys() -> Vec<String> {
    DRAFT_APPS.with(|m| m.iter().map(|a| a.exe.to_string()).collect())
}

fn add_to_draft(app: AppItem) {
    if SINGLE.get() {
        DRAFT_APPS.with(|m| m.set_vec(vec![app]));
        return;
    }
    DRAFT_APPS.with(|m| {
        let exists = m.iter().any(|a| a.exe.eq_ignore_ascii_case(&app.exe));
        if !exists {
            m.push(app);
        }
    });
}

fn item_for_key(key: &str) -> AppItem {
    match ICONS.with_borrow(|m| m.get(&key.to_lowercase()).cloned()) {
        Some((title, icon)) => app_item(&title, key, "", false, Some(icon)),
        None => app_item(key, key, "", false, None),
    }
}

/// Discovery may finish after the editor opened; fill in icons that are now known.
fn refresh_draft_icons() {
    DRAFT_APPS.with(|m| {
        for i in 0..m.row_count() {
            let Some(row) = m.row_data(i) else { continue };
            if !row.has_icon {
                let updated = item_for_key(&row.exe);
                if updated.has_icon {
                    m.set_row_data(i, updated);
                }
            }
        }
    });
}
