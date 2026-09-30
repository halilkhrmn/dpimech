//! System tray icon: show/quit plus a toggle per profile.
//!
//! On Linux the tray is a GTK object: GTK must be initialised on the thread that creates it,
//! and that thread must run the GTK main loop. Slint runs its own loop on the main thread, so
//! the tray gets a dedicated GTK thread and profile updates are sent to it.

use crate::i18n::tr;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use dpimech_core::model::ProfileState;
use slint::{ComponentHandle, Weak};
use tokio::sync::mpsc;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::AppWindow;
use crate::bridge::Command;

const SHOW_ID: &str = "show";
const QUIT_ID: &str = "quit";
const REPORT_ID: &str = "report";

thread_local! {
    static TRAY: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
}

/// Menu item id → (profile id, currently active). Read from the menu event thread.
static PROFILE_ITEMS: LazyLock<Mutex<HashMap<MenuId, (String, bool)>>> =
    LazyLock::new(Mutex::default);

fn build_tray() -> anyhow::Result<TrayIcon> {
    Ok(TrayIconBuilder::new()
        .with_tooltip("DPIMech")
        .with_icon(icon(IDLE))
        .with_menu(Box::new(build_menu(&[])?))
        .with_menu_on_left_click(false)
        .build()?)
}

#[cfg(target_os = "linux")]
static UPDATES: std::sync::OnceLock<Mutex<std::sync::mpsc::Sender<Vec<ProfileState>>>> =
    std::sync::OnceLock::new();

#[cfg(target_os = "linux")]
fn spawn_tray_thread() {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<ProfileState>>();
    let _ = UPDATES.set(Mutex::new(tx));
    std::thread::spawn(move || {
        if let Err(e) = gtk::init() {
            eprintln!("tray unavailable (GTK: {e})");
            return;
        }
        match build_tray() {
            Ok(tray) => TRAY.set(Some(tray)),
            Err(e) => {
                eprintln!("tray unavailable: {e:#}");
                return;
            }
        }
        // Only the newest profile list matters; older ones are skipped.
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            if let Some(profiles) = rx.try_iter().last() {
                apply_profiles(&profiles);
            }
            gtk::glib::ControlFlow::Continue
        });
        gtk::main();
    });
}

pub fn create(ui: Weak<AppWindow>, commands: mpsc::UnboundedSender<Command>) -> anyhow::Result<()> {
    // A missing tray (no GTK, no StatusNotifier host) must not keep the window from opening.
    #[cfg(target_os = "linux")]
    spawn_tray_thread();
    #[cfg(not(target_os = "linux"))]
    TRAY.set(Some(build_tray()?));

    let menu_ui = ui.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.as_ref() {
        SHOW_ID => show_window(&menu_ui),
        REPORT_ID => {
            let _ = menu_ui.upgrade_in_event_loop(|ui| ui.invoke_open_report());
        }
        QUIT_ID => {
            let _ = slint::invoke_from_event_loop(|| {
                let _ = slint::quit_event_loop();
            });
        }
        _ => {
            let target = PROFILE_ITEMS.lock().unwrap().get(&event.id).cloned();
            if let Some((id, active)) = target {
                let _ = commands.send(if active {
                    Command::Stop(id)
                } else {
                    Command::Start(id)
                });
            }
        }
    }));

    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            show_window(&ui);
        }
    }));
    Ok(())
}

/// Rebuilds the menu so each profile shows its current on/off state.
pub fn update_profiles(profiles: &[ProfileState]) {
    #[cfg(target_os = "linux")]
    if let Some(tx) = UPDATES.get() {
        let _ = tx.lock().unwrap().send(profiles.to_vec());
    }
    #[cfg(not(target_os = "linux"))]
    apply_profiles(profiles);
}

fn apply_profiles(profiles: &[ProfileState]) {
    let Ok(menu) = build_menu(profiles) else {
        return;
    };
    let active = profiles.iter().filter(|p| p.status.is_active()).count();
    let color = overall_color(profiles);
    TRAY.with_borrow(|tray| {
        if let Some(tray) = tray {
            tray.set_menu(Some(Box::new(menu)));
            let _ = tray.set_icon(Some(icon(color)));
            let tooltip = match active {
                0 => format!("DPIMech — {}", tr("idle")),
                n => format!("DPIMech — {}", trf!("{} profile(s) running", n)),
            };
            let _ = tray.set_tooltip(Some(tooltip));
        }
    });
}

fn build_menu(profiles: &[ProfileState]) -> anyhow::Result<Menu> {
    let menu = Menu::new();
    menu.append(&MenuItem::with_id(SHOW_ID, tr("Show DPIMech"), true, None))?;
    let mut items = PROFILE_ITEMS.lock().unwrap();
    items.clear();
    if !profiles.is_empty() {
        menu.append(&PredefinedMenuItem::separator())?;
        for p in profiles {
            let active = p.status.is_active();
            let item = CheckMenuItem::new(&p.profile.name, true, active, None);
            items.insert(item.id().clone(), (p.profile.id.clone(), active));
            menu.append(&item)?;
        }
    }
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&MenuItem::with_id(
        REPORT_ID,
        tr("Report a problem…"),
        true,
        None,
    ))?;
    menu.append(&MenuItem::with_id(QUIT_ID, tr("Quit"), true, None))?;
    Ok(menu)
}

fn show_window(ui: &Weak<AppWindow>) {
    let _ = ui.upgrade_in_event_loop(|ui| {
        let _ = ui.show();
    });
}

const IDLE: [u8; 3] = [0x8b, 0x94, 0x9e];
const OK: [u8; 3] = [0x2e, 0xa0, 0x43];
const SLOW: [u8; 3] = [0xd2, 0x99, 0x22];
const FAILED: [u8; 3] = [0xe5, 0x53, 0x4b];

/// Green when something runs fine, amber when a running profile is slow, red on errors,
/// grey when nothing runs — visible at a glance in the notification area.
fn overall_color(profiles: &[ProfileState]) -> [u8; 3] {
    use dpimech_core::model::ProfileStatus;
    let mut color = IDLE;
    for p in profiles {
        match &p.status {
            ProfileStatus::Error { .. } => return FAILED,
            ProfileStatus::Running {
                health: Some(h), ..
            } if h.slow || h.advice.is_some() => color = SLOW,
            ProfileStatus::Running { .. } | ProfileStatus::Starting if color == IDLE => color = OK,
            _ => {}
        }
    }
    color
}

/// The 32 px logo (raw RGBA from tools/make_icon.py, so no image decoder is needed) with a
/// status dot in the bottom-right corner: the octopus stays recognisable, the dot tells the
/// state at a glance.
fn icon(color: [u8; 3]) -> Icon {
    const SIZE: u32 = 32;
    const LOGO: &[u8; (SIZE * SIZE * 4) as usize] = include_bytes!("../assets/tray-32.rgba");
    let mut rgba = LOGO.to_vec();
    // Dot of radius 6 centred at (25, 25) with a 1.5 px white ring so it reads on any panel.
    let (cx, cy, r) = (25.0f32, 25.0f32, 6.0f32);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let pixel = if d <= r - 1.5 {
                [color[0], color[1], color[2], 0xff]
            } else if d <= r {
                [0xff, 0xff, 0xff, 0xff]
            } else {
                continue;
            };
            let i = ((y * SIZE + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&pixel);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("valid icon")
}
