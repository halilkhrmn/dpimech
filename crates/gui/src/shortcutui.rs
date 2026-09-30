//! "Create shortcut" dialog state (UI thread); the files are written by `shortcut.rs`.

use std::cell::RefCell;

use slint::{ComponentHandle, Image, ModelRc, SharedString, VecModel};

use crate::AppWindow;
use crate::i18n::tr;
use crate::shortcut::{self, Choice};
use crate::state::PROFILES;

thread_local! {
    /// Apps offered in the dialog, in the order of the combo box (without the last "nothing").
    static CHOICES: RefCell<Vec<Choice>> = const { RefCell::new(Vec::new()) };
    /// Profile the dialog is for; late discovery results for another profile are dropped.
    static PROFILE: RefCell<Option<String>> = const { RefCell::new(None) };
}

const PREVIEW: u32 = 160;

pub fn init(ui: &AppWindow) {
    ui.set_shortcut_menu_label(
        if cfg!(windows) {
            tr("Start menu")
        } else if cfg!(target_os = "macos") {
            tr("Applications folder")
        } else {
            tr("Applications menu")
        }
        .into(),
    );
}

pub fn open(ui: &AppWindow, id: &str) {
    let Some(profile) = PROFILES.with_borrow(|p| {
        p.iter()
            .find(|s| s.profile.id == id)
            .map(|s| s.profile.clone())
    }) else {
        return;
    };
    PROFILE.set(Some(id.to_owned()));
    CHOICES.set(Vec::new());
    ui.set_shortcut_profile_name(profile.name.clone().into());
    ui.set_shortcut_name(profile.name.clone().into());
    ui.set_shortcut_apps(apps_model(&[]));
    ui.set_shortcut_app_index(0);
    ui.set_shortcut_preview(Image::from_rgba8(shortcut::icon(None, PREVIEW)));
    ui.set_shortcut_loading(true);
    ui.set_shortcut_status(SharedString::new());
    ui.set_shortcut_ok(false);
    ui.set_shortcut_busy(false);
    ui.set_shortcut_open(true);
    let _ = ui.show();

    let weak = ui.as_weak();
    let id = id.to_owned();
    std::thread::spawn(move || {
        let choices = shortcut::choices(&profile);
        let icon = choices.first().and_then(shortcut::app_icon);
        let _ = weak.upgrade_in_event_loop(move |ui| {
            if PROFILE.with_borrow(|p| p.as_deref() != Some(&id)) {
                return;
            }
            ui.set_shortcut_apps(apps_model(&choices));
            ui.set_shortcut_app_index(0);
            ui.set_shortcut_preview(Image::from_rgba8(shortcut::icon(icon.as_ref(), PREVIEW)));
            ui.set_shortcut_loading(false);
            CHOICES.set(choices);
        });
    });
}

pub fn app_changed(ui: &AppWindow, index: i32) {
    let icon = selected(index).and_then(|c| shortcut::app_icon(&c));
    ui.set_shortcut_preview(Image::from_rgba8(shortcut::icon(icon.as_ref(), PREVIEW)));
    ui.set_shortcut_status(SharedString::new());
    ui.set_shortcut_ok(false);
}

pub fn create(ui: &AppWindow) {
    let Some(profile_id) = PROFILE.with_borrow(|p| p.clone()) else {
        return;
    };
    let request = shortcut::Request {
        profile_id,
        name: ui.get_shortcut_name().trim().to_owned(),
        open: selected(ui.get_shortcut_app_index()),
        desktop: ui.get_shortcut_desktop(),
        menu: ui.get_shortcut_menu(),
    };
    ui.set_shortcut_busy(true);
    ui.set_shortcut_status(SharedString::new());
    let weak = ui.as_weak();
    // PowerShell (Windows) and icon extraction take a moment: keep the window responsive.
    std::thread::spawn(move || {
        let result = shortcut::create(&request);
        let _ = weak.upgrade_in_event_loop(move |ui| {
            ui.set_shortcut_busy(false);
            match result {
                Ok(paths) => {
                    let list = paths
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.set_shortcut_status(trf!("Shortcut created:\n{}", list).into());
                    ui.set_shortcut_ok(true);
                }
                Err(e) => {
                    ui.set_shortcut_status(trf!("Could not create the shortcut: {}", e).into());
                    ui.set_shortcut_ok(false);
                }
            }
        });
    });
}

fn selected(index: i32) -> Option<Choice> {
    let index = usize::try_from(index).ok()?;
    CHOICES.with_borrow(|c| c.get(index).cloned())
}

fn apps_model(choices: &[Choice]) -> ModelRc<SharedString> {
    let mut names: Vec<SharedString> = choices.iter().map(|c| c.title.as_str().into()).collect();
    names.push(tr("Nothing, only turn the profile on").into());
    ModelRc::new(VecModel::from(names))
}
