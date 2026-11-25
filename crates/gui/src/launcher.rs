//! What a profile shortcut runs: `dpimech --launch <profile> [--open <app>]`.
//! It switches the profile on, opens the app and exits; one system notification shows the
//! progress and the result (no window of its own). It never takes the single-instance lock: the
//! tray and hotkey belong to the main app, which it starts in the tray when it is not running yet.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use dpimech_core::ipc::{Client, Reply, Request};
use dpimech_core::model::{ProfileStatus, Routing};

use crate::i18n::tr;
use crate::notify::TaskNote;
use crate::shortcut::{self, Choice};
use crate::{appproxy, autostart, prefs, single};

/// Engines normally come up in a second or two; ProxiFyre and the driver can take longer.
const START_TIMEOUT: Duration = Duration::from_secs(60);

/// Parses `--launch <id> [--open <target>]`; `None` for a normal start.
pub fn from_args(args: &[String]) -> Option<(String, Option<String>)> {
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let id = value(shortcut::LAUNCH_FLAG)?;
    Some((id, value(shortcut::OPEN_FLAG).filter(|t| !t.is_empty())))
}

/// The notification of one launch. It appears once the profile's name is known, titled with it;
/// an error before that (no service, profile deleted) is titled "DPIMech".
struct Steps {
    title: Option<String>,
    icon: Option<PathBuf>,
    note: Option<TaskNote>,
}

impl Steps {
    fn note(&mut self) -> &mut TaskNote {
        let (title, icon) = (&self.title, &self.icon);
        self.note.get_or_insert_with(|| {
            TaskNote::new(title.as_deref().unwrap_or("DPIMech"), icon.clone())
        })
    }

    fn step(&mut self, status: &str, value: f32) {
        self.note().update(&tr("Starting…"), status, value);
    }
}

pub fn run(profile_id: String, open: Option<String>) -> anyhow::Result<()> {
    crate::i18n::apply(&prefs::load().language);
    // The launcher is its own process: it needs the app's notification identity too.
    crate::notify::register_identity();
    let mut steps = Steps {
        title: None,
        icon: launch_icon(open.as_deref()),
        note: None,
    };
    // Without the main app there is no tray to switch the profile off again.
    if !single::is_running() {
        start_main(&[autostart::MINIMIZED_FLAG]);
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(start_profile(&mut steps, &profile_id));
    let result = result.and_then(|(name, routing)| {
        let target = match (&routing, &open) {
            // The app only uses the engine when DPIMech opens it with the proxy switch.
            (Routing::AppProxy { app, port }, _) => Some((app.clone(), Some(*port))),
            (_, Some(target)) => Some((target.clone(), None)),
            (_, None) => None,
        };
        let Some((target, port)) = target else {
            return Ok(name);
        };
        steps.step(&trf!("Opening {}…", display_name(&target)), 0.8);
        match port {
            Some(port) => appproxy::open(&target, port),
            None => open_target(&target),
        }
        .map(|()| name)
        .map_err(|e| trf!("Could not open {}: {}", display_name(&target), e))
    });
    match result {
        Ok(name) => {
            steps.note().finish(&trf!("{} is on", name), "", false);
        }
        Err(message) => {
            if !steps.note().finish(&tr("Could not start"), &message, true) {
                // No notification could be shown: an error must still be seen.
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title(tr("Could not start"))
                    .set_description(message)
                    .show();
            }
        }
    }
    Ok(())
}

/// The shortcut's picture (the logo with the app's icon) as a file the notification can show.
fn launch_icon(open: Option<&str>) -> Option<PathBuf> {
    let app_icon = open.and_then(|target| {
        shortcut::app_icon(&Choice {
            title: String::new(),
            launch: target.to_owned(),
            icon_source: target.to_owned(),
        })
    });
    let path = prefs::config_file("launch-icon.png")?;
    std::fs::create_dir_all(path.parent()?).ok()?;
    let png = shortcut::encode_png(&shortcut::icon(app_icon.as_ref(), 128));
    std::fs::write(&path, png).ok()?;
    Some(path)
}

/// Switches the profile on (if it is not already) and waits until it runs.
/// Returns the profile's name and routing.
async fn start_profile(steps: &mut Steps, id: &str) -> Result<(String, Routing), String> {
    let (client, _events) = Client::connect()
        .await
        .map_err(|_| tr("The DPIMech service is not running. Open DPIMech to set it up."))?;
    let find = |reply| match reply {
        Ok(Reply::Profiles { profiles }) => profiles.into_iter().find(|p| p.profile.id == id),
        _ => None,
    };
    let state = find(client.request(Request::ListProfiles).await)
        .ok_or_else(|| tr("The profile of this shortcut no longer exists."))?;
    let name = state.profile.name.clone();
    let routing = state.profile.routing.clone();
    steps.title = Some(name.clone());
    if matches!(state.status, ProfileStatus::Running { .. }) {
        return Ok((name, routing));
    }

    steps.step(&tr("Turning the profile on…"), 0.3);
    client
        .request(Request::StartProfile { id: id.to_owned() })
        .await
        .map_err(|e| e.to_string())?;
    // Polling keeps this simple and cannot miss a status change that happened before the
    // first event arrived.
    let started = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_millis(400)).await;
        let state = find(client.request(Request::ListProfiles).await)
            .ok_or_else(|| tr("The profile of this shortcut no longer exists."))?;
        match state.status {
            ProfileStatus::Running { .. } => return Ok((name, routing)),
            ProfileStatus::Error { message } => return Err(message),
            ProfileStatus::Stopped if started.elapsed() > Duration::from_secs(5) => {
                return Err(tr(
                    "The profile stopped right after starting. The Logs page in DPIMech shows why.",
                ));
            }
            _ if started.elapsed() > START_TIMEOUT => {
                return Err(tr(
                    "The profile is taking too long to start. The Logs page in DPIMech shows why.",
                ));
            }
            _ => {}
        }
    }
}

/// Starts the main app. From an AppImage this must be the image itself: the mount this process
/// runs from goes away when it exits.
fn start_main(args: &[&str]) {
    use std::process::{Command, Stdio};
    match shortcut::launcher_exe() {
        Ok(exe) => {
            let spawned = Command::new(exe)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            if let Err(e) = spawned {
                eprintln!("starting DPIMech: {e}");
            }
        }
        Err(e) => eprintln!("{e:#}"),
    }
}

/// "Discord" for `…\Discord.lnk`, `/usr/share/applications/discord.desktop`, `…/Discord.exe`.
pub fn display_name(target: &str) -> String {
    #[cfg(all(unix, not(target_os = "macos")))]
    if target.ends_with(".desktop")
        && let Ok(text) = std::fs::read_to_string(target)
        && let Some(name) = shortcut::linux::desktop_value(&text, "Name")
    {
        return name;
    }
    // Split by hand: the target may use either separator whatever the OS.
    let file = target
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(target);
    match file.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_owned(),
        _ => file.to_owned(),
    }
}

#[cfg(windows)]
fn open_target(target: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (verb, file) = (wide("open"), wide(target));
    let dir = std::path::Path::new(target)
        .parent()
        .map(|d| wide(&d.display().to_string()))
        .unwrap_or_else(|| vec![0]);
    // SAFETY: all strings are NUL-terminated and outlive the call.
    let code = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            dir.as_ptr(),
            SW_SHOWNORMAL,
        )
    } as isize;
    // ShellExecute reports success as a value above 32.
    if code > 32 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_target(target: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};

    let quiet = |cmd: &mut Command| {
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    };
    if target.ends_with(".desktop") {
        // gio and gtk-launch start the app the way the desktop's own menu does.
        if quiet(Command::new("gio").args(["launch", target])) {
            return Ok(());
        }
        let id = std::path::Path::new(target)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if quiet(Command::new("gtk-launch").arg(&id)) {
            return Ok(());
        }
        let text = std::fs::read_to_string(target).map_err(|e| e.to_string())?;
        let exec = shortcut::linux::desktop_value(&text, "Exec")
            .ok_or_else(|| tr("the menu entry has no command"))?;
        let words: Vec<String> = dpimech_core::args::split_args(&exec)
            .into_iter()
            .filter(|w| !(w.starts_with('%') && w.len() == 2))
            .collect();
        let (program, args) = words
            .split_first()
            .ok_or_else(|| tr("the menu entry has no command"))?;
        return Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .spawn()
            .map(drop)
            .map_err(|e| e.to_string());
    }
    Command::new(target)
        .stdin(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn open_target(target: &str) -> Result<(), String> {
    let status = std::process::Command::new("open")
        .arg(target)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(status.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_arguments() {
        let args: Vec<String> = ["dpimech", "--launch", "p1", "--open", "C:\\x\\Discord.lnk"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            from_args(&args),
            Some(("p1".into(), Some("C:\\x\\Discord.lnk".into())))
        );
        assert_eq!(from_args(&args[..3]), Some(("p1".into(), None)));
        assert_eq!(from_args(&["dpimech".to_string()]), None);
        assert_eq!(display_name("C:\\x\\Discord.lnk"), "Discord");
    }
}
