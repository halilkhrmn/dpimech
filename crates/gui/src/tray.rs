//! System tray icon: show/quit plus a toggle per profile.
//!
//! On Linux the tray is a GTK object: GTK must be initialised on the thread that creates it,
//! and that thread must run the GTK main loop. Slint runs its own loop on the main thread, so
//! the tray gets a dedicated GTK thread and profile updates are sent to it.

use crate::i18n::tr;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use dpimech_core::model::ProfileState;
use slint::Weak;
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
    #[cfg(not(target_os = "linux"))]
    static BLINK_TIMER: slint::Timer = slint::Timer::default();
}

/// Menu item id → (profile id, currently active). Read from the menu event thread.
static PROFILE_ITEMS: LazyLock<Mutex<HashMap<MenuId, (String, bool)>>> =
    LazyLock::new(Mutex::default);

const TRAY_PENDING: u8 = 0;
const TRAY_READY: u8 = 1;
const TRAY_FAILED: u8 = 2;

/// Whether the tray icon exists. Without it, closing the window must quit: a hidden window could
/// not be brought back.
static STATE: AtomicU8 = AtomicU8::new(TRAY_PENDING);

/// Until when (unix seconds) a profile counts as connecting: starting, or running without the
/// result of its first connection check. A time, not a flag, so a profile without checks does
/// not keep the dot blinking.
static CONNECTING_UNTIL: AtomicU64 = AtomicU64::new(0);
/// The Strategy Lab (or the setup wizard through it) is testing.
static TESTING: AtomicBool = AtomicBool::new(false);
/// Dot colour for the current profiles, kept for when the blinking stops.
static STEADY: Mutex<[u8; 3]> = Mutex::new(IDLE);
/// When the current busy spell began; `None` while nothing is busy.
static BUSY_SINCE: Mutex<Option<Instant>> = Mutex::new(None);
/// After this the dot stays amber instead of blinking: a long test should not flash forever.
const BLINK_FOR: Duration = Duration::from_secs(120);
const BLINK_EVERY: Duration = Duration::from_millis(600);
/// How long after a start the first connection check counts as "still connecting".
const CONNECTING_FOR: u64 = 30;

/// Tests started or finished (Strategy Lab, setup wizard).
pub fn set_testing(on: bool) {
    TESTING.store(on, Ordering::Relaxed);
}

fn busy() -> bool {
    unix_now() < CONNECTING_UNTIL.load(Ordering::Relaxed) || TESTING.load(Ordering::Relaxed)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Called every `BLINK_EVERY` on the tray's thread: alternates the amber dot with no dot
/// while something is connecting or testing, and puts the steady icon back afterwards.
fn blink_tick(phase: &mut bool) {
    let mut since = BUSY_SINCE.lock().unwrap();
    let icon = if busy() {
        let started = *since.get_or_insert_with(Instant::now);
        *phase = !*phase;
        if started.elapsed() > BLINK_FOR || *phase {
            icon(SLOW)
        } else {
            plain_icon()
        }
    } else if since.take().is_some() {
        icon(*STEADY.lock().unwrap())
    } else {
        return;
    };
    TRAY.with_borrow(|tray| {
        if let Some(tray) = tray {
            let _ = tray.set_icon(Some(icon));
        }
    });
}

pub fn available() -> bool {
    ready() == Some(true)
}

/// `None` while the tray is still coming up (Linux builds it on its own thread).
pub fn ready() -> Option<bool> {
    match STATE.load(Ordering::Relaxed) {
        TRAY_PENDING => None,
        state => Some(state == TRAY_READY),
    }
}

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
            STATE.store(TRAY_FAILED, Ordering::Relaxed);
            return;
        }
        crate::bundled::load(crate::bundled::INDICATOR);
        // The appindicator binding panics when no libayatana-appindicator3 can be loaded.
        match std::panic::catch_unwind(build_tray) {
            Ok(Ok(tray)) => TRAY.set(Some(tray)),
            Ok(Err(e)) => {
                eprintln!("tray unavailable: {e:#}");
                STATE.store(TRAY_FAILED, Ordering::Relaxed);
                return;
            }
            Err(_) => {
                eprintln!("tray unavailable: libayatana-appindicator3 could not be loaded");
                STATE.store(TRAY_FAILED, Ordering::Relaxed);
                return;
            }
        }
        STATE.store(TRAY_READY, Ordering::Relaxed);
        let mut phase = false;
        gtk::glib::timeout_add_local(BLINK_EVERY, move || {
            blink_tick(&mut phase);
            gtk::glib::ControlFlow::Continue
        });
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
    match build_tray() {
        Ok(tray) => {
            TRAY.set(Some(tray));
            STATE.store(TRAY_READY, Ordering::Relaxed);
            // The tray belongs to this (the UI) thread, so the blink timer runs here too.
            BLINK_TIMER.with(|timer| {
                let mut phase = false;
                timer.start(slint::TimerMode::Repeated, BLINK_EVERY, move || {
                    blink_tick(&mut phase);
                });
            });
        }
        Err(e) => {
            STATE.store(TRAY_FAILED, Ordering::Relaxed);
            return Err(e);
        }
    }

    let menu_ui = ui.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.as_ref() {
        SHOW_ID => show_window(&menu_ui),
        REPORT_ID => {
            let _ = menu_ui.upgrade_in_event_loop(|ui| {
                ui.invoke_open_report();
                crate::bring_up(&ui);
            });
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
    *STEADY.lock().unwrap() = color;
    CONNECTING_UNTIL.store(connecting_until(profiles), Ordering::Relaxed);
    TRAY.with_borrow(|tray| {
        if let Some(tray) = tray {
            tray.set_menu(Some(Box::new(menu)));
            // While busy the blink timer owns the icon.
            if !busy() {
                let _ = tray.set_icon(Some(icon(color)));
            }
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
    let _ = ui.upgrade_in_event_loop(|ui| crate::bring_up(&ui));
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

/// Until when the profiles count as connecting: a starting one for the next
/// `CONNECTING_FOR` seconds, a running one without a check result until `CONNECTING_FOR`
/// after its start. 0 when none is.
fn connecting_until(profiles: &[ProfileState]) -> u64 {
    use dpimech_core::model::ProfileStatus;
    let now = unix_now();
    profiles
        .iter()
        .filter_map(|p| match &p.status {
            ProfileStatus::Starting => Some(now + CONNECTING_FOR),
            ProfileStatus::Running {
                health: None,
                since_unix,
                ..
            } => Some(since_unix + CONNECTING_FOR),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

const SIZE: u32 = 32;
const LOGO: &[u8; (SIZE * SIZE * 4) as usize] = include_bytes!("../assets/tray-32.rgba");

/// The logo without a dot: the "off" half of the blink.
fn plain_icon() -> Icon {
    Icon::from_rgba(LOGO.to_vec(), SIZE, SIZE).expect("valid icon")
}

/// The 32 px logo (raw RGBA from tools/make_icon.py, so no image decoder is needed) with a
/// status dot in the bottom-right corner: the chameleon stays recognisable, the dot tells the
/// state at a glance.
fn icon(color: [u8; 3]) -> Icon {
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpimech_core::model::{
        ConnectionHealth, EngineKind, Profile, ProfileStatus, Reliability, Routing,
    };

    fn state(status: ProfileStatus) -> ProfileState {
        ProfileState {
            profile: Profile {
                id: "p".into(),
                name: "p".into(),
                engine: EngineKind::ByeDpi,
                args: String::new(),
                routing: Routing::LocalProxy { port: 1080 },
                autostart: false,
                reliability: Reliability::default(),
                check_sites: Vec::new(),
                per_network: Default::default(),
            },
            status,
        }
    }

    #[test]
    fn connecting_ends_with_the_first_check_or_after_a_while() {
        let now = unix_now();
        let running = |since_unix, health| ProfileStatus::Running {
            since_unix,
            restarts: 0,
            health,
        };
        // Starting: connecting for a while from now.
        let until = connecting_until(&[state(ProfileStatus::Starting)]);
        assert!(until >= now + CONNECTING_FOR);
        // Just started, no check yet: connecting until CONNECTING_FOR after the start.
        assert_eq!(
            connecting_until(&[state(running(now - 5, None))]),
            now - 5 + CONNECTING_FOR
        );
        // A profile without checks stops counting once that time has passed.
        let old = connecting_until(&[state(running(now - 3600, None))]);
        assert!(old < now);
        // The first check came: no longer connecting.
        let checked = ConnectionHealth {
            latency_ms: 80,
            usual_ms: 0,
            ok: 2,
            total: 2,
            checked_unix: now,
            slow: false,
            advice: None,
        };
        assert_eq!(connecting_until(&[state(running(now, Some(checked)))]), 0);
        assert_eq!(connecting_until(&[state(ProfileStatus::Stopped)]), 0);
    }

    #[test]
    fn a_test_makes_the_tray_busy() {
        set_testing(true);
        assert!(busy());
        set_testing(false);
        assert!(!busy());
    }
}
