//! Windows notifications for things the user should know about without opening the window:
//! a profile got slow, was restarted, stopped with an error, or needs a new strategy.

use crate::i18n::tr;
use std::collections::HashMap;
use std::sync::Mutex;

use dpimech_core::model::{ProfileState, ProfileStatus};

static PREVIOUS: Mutex<Option<HashMap<String, ProfileStatus>>> = Mutex::new(None);

/// Compares with the last known statuses and shows a toast for meaningful changes.
/// The very first list (app start) only records the state. Returns the names of profiles
/// that have just stopped with an error.
pub fn on_profiles(profiles: &[ProfileState]) -> Vec<String> {
    let mut guard = PREVIOUS.lock().unwrap();
    let first = guard.is_none();
    let previous = guard.get_or_insert_with(HashMap::new);
    let mut failed = Vec::new();
    for p in profiles {
        let before = previous.insert(p.profile.id.clone(), p.status.clone());
        if first {
            continue;
        }
        if matches!(p.status, ProfileStatus::Error { .. })
            && !matches!(before, Some(ProfileStatus::Error { .. }))
        {
            failed.push(p.profile.name.clone());
        }
        if let Some((title, body)) = describe(&p.profile.name, before.as_ref(), &p.status) {
            show(&title, &body);
        }
    }
    failed
}

fn describe(
    name: &str,
    before: Option<&ProfileStatus>,
    now: &ProfileStatus,
) -> Option<(String, String)> {
    let health = |s: Option<&ProfileStatus>| match s {
        Some(ProfileStatus::Running { health, .. }) => health.clone(),
        _ => None,
    };
    let restarts = |s: Option<&ProfileStatus>| match s {
        Some(ProfileStatus::Running { restarts, .. }) => *restarts,
        _ => 0,
    };
    match now {
        ProfileStatus::Error { message }
            if !matches!(before, Some(ProfileStatus::Error { .. })) =>
        {
            Some((trf!("{} stopped", name), message.clone()))
        }
        ProfileStatus::Running { .. } => {
            let (old, new) = (health(before), health(Some(now)));
            let was_slow = old.as_ref().is_some_and(|h| h.slow);
            if let Some(h) = &new {
                if h.advice.is_some() && old.as_ref().is_none_or(|o| o.advice.is_none()) {
                    return Some((
                        trf!("{} needs attention", name),
                        tr(h.advice.as_deref().unwrap_or_default()),
                    ));
                }
                if h.slow && !was_slow {
                    let what = if h.ok == 0 {
                        tr("the sites stopped answering")
                    } else if h.usual_ms > 0 {
                        trf!("{} ms instead of the usual {} ms", h.latency_ms, h.usual_ms)
                    } else {
                        trf!("only {}/{} sites answer", h.ok, h.total)
                    };
                    return Some((
                        trf!("{} is slow", name),
                        trf!("Connection check: {}.", what),
                    ));
                }
            }
            if restarts(Some(now)) > restarts(before) && before.is_some() {
                return Some((
                    trf!("{} was restarted", name),
                    tr("DPIMech restarted the engine to fix the connection."),
                ));
            }
            None
        }
        _ => None,
    }
}

/// Our AppUserModelID. Windows shows its registered name and icon on every toast.
#[cfg(windows)]
const APP_ID: &str = "dpimech.app";

/// Registers the app's notification identity for the current user (no installer or admin
/// needed) and tags this process with it, so toasts say "DPIMech" with our logo instead of
/// borrowing another app's identity.
#[cfg(windows)]
pub fn register_identity() {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let icon = std::env::var_os("LOCALAPPDATA").map(|base| {
        std::path::PathBuf::from(base)
            .join("dpimech")
            .join("dpimech.png")
    });
    if let Some(icon) = &icon
        && let Some(dir) = icon.parent()
    {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(icon, include_bytes!("../assets/dpimech.png"));
    }
    if let Ok((key, _)) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(format!(r"Software\Classes\AppUserModelId\{APP_ID}"))
    {
        let _ = key.set_value("DisplayName", &"DPIMech");
        if let Some(icon) = &icon {
            let _ = key.set_value("IconUri", &icon.display().to_string());
        }
    }
    // 0.1.x registered "dpimngr.app"; its entry and icon copy are no longer used.
    let _ = RegKey::predef(HKEY_CURRENT_USER)
        .delete_subkey_all(r"Software\Classes\AppUserModelId\dpimngr.app");
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        let _ = std::fs::remove_dir_all(std::path::PathBuf::from(base).join("dpimngr"));
    }
    let wide: Vec<u16> = APP_ID.encode_utf16().chain(Some(0)).collect();
    // SAFETY: NUL-terminated UTF-16 string valid for the duration of the call.
    unsafe {
        windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
    }
}

/// Puts the logo into the user's icon theme so notifications and the autostart entry can
/// refer to it as `dpimech`.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn register_identity() {
    let Some(base) = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })
    else {
        return;
    };
    let icon = base.join("icons/hicolor/256x256/apps/dpimech.png");
    if icon.exists() {
        return;
    }
    if let Some(dir) = icon.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(icon, include_bytes!("../assets/dpimech.png"));
}

#[cfg(target_os = "macos")]
pub fn register_identity() {}

/// A profile with "Open with proxy" is on, but its app could not be opened.
pub fn app_open_failed(app: &str, error: &str) {
    show(&trf!("Could not open {}", app), error);
}

/// Tells the user once per version that a new DPIMech release is out.
pub fn app_update(version: &str) {
    static SHOWN: Mutex<Option<String>> = Mutex::new(None);
    let mut shown = SHOWN.lock().unwrap();
    if shown.as_deref() != Some(version) {
        *shown = Some(version.to_owned());
        show(
            &tr("DPIMech update available"),
            &trf!(
                "Version {} is out. Settings → About shows how to get it.",
                version
            ),
        );
    }
}

/// The update is downloaded and waits for a restart; once per version.
pub fn update_ready(version: &str) {
    static SHOWN: Mutex<Option<String>> = Mutex::new(None);
    let mut shown = SHOWN.lock().unwrap();
    if shown.as_deref() != Some(version) {
        *shown = Some(version.to_owned());
        show(
            &trf!("DPIMech {} is ready", version),
            &tr("Restart DPIMech to update: use the button at the top of the window."),
        );
    }
}

/// Development aid: `DPIMECH_DEBUG_TOAST=1` shows a sample notification at start.
#[cfg(debug_assertions)]
pub fn show_test() {
    show("DPIMech", "Notifications show DPIMech's name and icon.");
}

#[cfg(windows)]
fn show(title: &str, body: &str) {
    use tauri_winrt_notification::Toast;
    if let Err(e) = Toast::new(APP_ID).title(title).text1(body).show() {
        eprintln!("notification failed: {e:?}");
    }
}

/// Desktop notification through the freedesktop notification service; `notify-send` ships
/// with every common desktop and avoids a D-Bus client in the GUI.
#[cfg(all(unix, not(target_os = "macos")))]
fn show(title: &str, body: &str) {
    let spawned = std::process::Command::new("notify-send")
        .args(["--app-name=DPIMech", "--icon=dpimech", "--", title, body])
        .stdin(std::process::Stdio::null())
        .spawn();
    match spawned {
        // Reap it in the background so no zombie is left behind.
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(e) => eprintln!("notification failed (is notify-send installed?): {e}"),
    }
}

/// macOS: Notification Center through AppleScript (no extra framework bindings needed).
#[cfg(target_os = "macos")]
fn show(title: &str, body: &str) {
    let quote = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        quote(body),
        quote(title)
    );
    let spawned = std::process::Command::new("osascript")
        .args(["-e", &script])
        .stdin(std::process::Stdio::null())
        .spawn();
    if let Ok(mut child) = spawned {
        std::thread::spawn(move || child.wait());
    }
}
