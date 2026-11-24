//! Windows notifications for things the user should know about without opening the window:
//! a profile got slow, was restarted, stopped with an error, or needs a new strategy.

use crate::i18n::tr;
use std::collections::HashMap;
use std::sync::Mutex;

use dpimech_core::model::{OFFLINE_ADVICE, ProfileState, ProfileStatus};

static PREVIOUS: Mutex<Option<HashMap<String, ProfileStatus>>> = Mutex::new(None);
/// Set while a lost connection was already announced: one notification for the whole
/// outage, not one per profile or per check.
static OFFLINE_SHOWN: Mutex<bool> = Mutex::new(false);

fn is_offline(status: &ProfileStatus) -> bool {
    matches!(status, ProfileStatus::Running { health: Some(h), .. }
        if h.advice.as_deref() == Some(OFFLINE_ADVICE))
}

/// Compares with the last known statuses and shows a toast for meaningful changes.
/// The very first list (app start) only records the state. Returns the names of profiles
/// that have just stopped with an error.
pub fn on_profiles(profiles: &[ProfileState]) -> Vec<String> {
    let mut guard = PREVIOUS.lock().unwrap();
    let first = guard.is_none();
    let previous = guard.get_or_insert_with(HashMap::new);
    let mut failed = Vec::new();
    let offline_now = profiles.iter().any(|p| is_offline(&p.status));
    {
        let mut shown = OFFLINE_SHOWN.lock().unwrap();
        if offline_now && !*shown && !first {
            show(&tr("No internet connection"), &tr(OFFLINE_ADVICE));
        }
        *shown = offline_now;
    }
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
        // Announced once above; per-profile "slow" and "restarted" toasts would only repeat it.
        if offline_now || before.as_ref().is_some_and(is_offline) {
            continue;
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
/// refer to it as `dpimech`. This copy wins over the packaged icon, so it is replaced when an
/// update brings a new logo.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn register_identity() {
    const LOGO: &[u8] = include_bytes!("../assets/dpimech.png");
    let Some(base) = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })
    else {
        return;
    };
    let theme = base.join("icons/hicolor");
    let icon = theme.join("256x256/apps/dpimech.png");
    if std::fs::read(&icon).is_ok_and(|old| old == LOGO) {
        return;
    }
    if let Some(dir) = icon.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::write(&icon, LOGO).is_ok() {
        // GTK notices icon theme changes by the theme folder's date.
        let _ =
            std::fs::File::open(&theme).and_then(|f| f.set_modified(std::time::SystemTime::now()));
    }
}

#[cfg(target_os = "macos")]
pub fn register_identity() {}

/// A profile with "Open with proxy" is on, but its app could not be opened.
pub fn app_open_failed(app: &str, error: &str) {
    show(&trf!("Could not open {}", app), error);
}

/// Closing the window only hides it; said once, the first time it happens.
pub fn still_in_tray() {
    show(
        &tr("DPIMech is still running"),
        &tr(
            "Profiles keep working. To stop them and quit, right-click the DPIMech icon next to the clock.",
        ),
    );
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

/// One system notification that follows a task (a profile shortcut starting its profile): shown
/// once, then updated in place, so the user sees a single native notification instead of a
/// window of our own. `title` (the profile) stays; `heading` and `status` change with the task.
pub struct TaskNote {
    title: String,
    #[cfg_attr(target_os = "macos", allow(dead_code))] // osascript cannot show our icon
    icon: Option<std::path::PathBuf>,
    shown: bool,
    /// Linux: the notification's id, to replace it (`notify-send -r`). `None` when this
    /// notify-send cannot replace (older libnotify): then only the result is shown.
    #[cfg(all(unix, not(target_os = "macos")))]
    id: Option<String>,
}

impl TaskNote {
    pub fn new(title: &str, icon: Option<std::path::PathBuf>) -> TaskNote {
        TaskNote {
            title: title.to_owned(),
            icon,
            shown: false,
            #[cfg(all(unix, not(target_os = "macos")))]
            id: None,
        }
    }

    /// Shows or updates the notification; `value` is the progress between 0 and 1.
    pub fn update(&mut self, heading: &str, status: &str, value: f32) {
        self.post(heading, status, value, false);
    }

    /// The task ended. Returns false when no notification could be shown at all, so the caller
    /// can fall back to a dialog for an error.
    pub fn finish(&mut self, heading: &str, status: &str, failed: bool) -> bool {
        // A failed task gets no bar: an empty one would look like it is still going.
        let value = if failed { -1.0 } else { 1.0 };
        self.post(heading, status, value, true)
    }

    #[cfg(windows)]
    fn post(&mut self, heading: &str, status: &str, value: f32, _last: bool) -> bool {
        use tauri_winrt_notification::{IconCrop, Progress, Toast};
        let progress = Progress {
            tag: "launch".into(),
            title: heading.into(),
            status: status.into(),
            value: value.max(0.0),
            value_string: String::new(),
        };
        if self.shown {
            return Toast::new(APP_ID).set_progress(&progress).is_ok();
        }
        let mut toast = Toast::new(APP_ID)
            .title(&self.title)
            .sound(None)
            .progress(&progress);
        if let Some(icon) = &self.icon {
            toast = toast.icon(icon, IconCrop::Square, "");
        }
        self.shown = toast.show().is_ok();
        self.shown
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn post(&mut self, heading: &str, status: &str, value: f32, last: bool) -> bool {
        // Without replacing, every step would pile up as its own notification: show the result only.
        if self.shown && self.id.is_none() && !last {
            return true;
        }
        let mut cmd = std::process::Command::new("notify-send");
        cmd.args(["--app-name=DPIMech", "--print-id"]);
        cmd.arg(format!(
            "--icon={}",
            self.icon
                .as_ref()
                .map_or("dpimech".into(), |p| p.display().to_string())
        ));
        if value >= 0.0 {
            cmd.arg(format!(
                "--hint=int:value:{}",
                (value * 100.0).round() as i32
            ));
        }
        if let Some(id) = &self.id {
            cmd.arg(format!("--replace-id={id}"));
        }
        let body = if status.is_empty() {
            heading.to_owned()
        } else {
            format!("{heading}\n{status}")
        };
        cmd.args(["--", &self.title, &body]);
        let Ok(out) = cmd.stdin(std::process::Stdio::null()).output() else {
            return false;
        };
        if !out.status.success() {
            // An older notify-send without --print-id / --replace-id: plain notification.
            let shown = std::process::Command::new("notify-send")
                .args([
                    "--app-name=DPIMech",
                    "--icon=dpimech",
                    "--",
                    &self.title,
                    &body,
                ])
                .status()
                .is_ok_and(|s| s.success());
            self.shown |= shown;
            return shown;
        }
        let id = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        self.id = (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).then_some(id);
        self.shown = true;
        true
    }

    /// Notification Center cannot update a notification: the first step and the result only.
    #[cfg(target_os = "macos")]
    fn post(&mut self, heading: &str, status: &str, _value: f32, last: bool) -> bool {
        if self.shown && !last {
            return true;
        }
        self.shown = true;
        let body = if status.is_empty() {
            heading.to_owned()
        } else {
            format!("{heading} — {status}")
        };
        show(&self.title, &body);
        true
    }
}
