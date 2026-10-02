//! What a profile shortcut runs: `dpimech --launch <profile> [--open <app>]`.
//! A small window shows progress while the profile is switched on, then opens the app and
//! closes itself. It never takes the single-instance lock: the tray and hotkey belong to the main
//! app, which it starts in the tray when it is not running yet.

use std::time::{Duration, Instant};

use dpimech_core::ipc::{Client, Reply, Request};
use dpimech_core::model::ProfileStatus;
use slint::{ComponentHandle, Image, SharedString, Weak};

use crate::i18n::tr;
use crate::shortcut::{self, Choice};
use crate::{LaunchWindow, autostart, prefs, single};

/// Engines normally come up in a second or two; ProxiFyre and the driver can take longer.
const START_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the "ready" state stays visible before the window closes.
const DONE_DELAY: Duration = Duration::from_millis(1400);

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

pub fn run(profile_id: String, open: Option<String>) -> anyhow::Result<()> {
    crate::i18n::apply(&prefs::load().language);
    let ui = LaunchWindow::new()?;
    let app_icon = open.as_deref().and_then(|target| {
        shortcut::app_icon(&Choice {
            title: String::new(),
            launch: target.to_owned(),
            icon_source: target.to_owned(),
        })
    });
    ui.set_picture(Image::from_rgba8(shortcut::icon(app_icon.as_ref(), 112)));
    ui.set_heading(tr("Starting…").into());
    ui.set_status(tr("Connecting to the DPIMech service…").into());
    ui.set_state("working".into());
    ui.on_dismiss(|| {
        let _ = slint::quit_event_loop();
    });
    ui.on_open_main(|| {
        start_main(&[]);
        let _ = slint::quit_event_loop();
    });
    // Without the main app there is no tray to switch the profile off again.
    if !single::is_running() {
        start_main(&[autostart::MINIMIZED_FLAG]);
    }

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime.block_on(start_profile(&weak, &profile_id));
        let result = result.and_then(|name| match &open {
            Some(target) => {
                set(&weak, None, Some(trf!("Opening {}…", display_name(target))));
                open_target(target)
                    .map(|()| name)
                    .map_err(|e| trf!("Could not open {}: {}", display_name(target), e))
            }
            None => Ok(name),
        });
        let _ = weak.upgrade_in_event_loop(move |ui| match result {
            Ok(name) => {
                ui.set_heading(trf!("{} is on", name).into());
                ui.set_status(SharedString::new());
                ui.set_state("done".into());
                slint::Timer::single_shot(DONE_DELAY, || {
                    let _ = slint::quit_event_loop();
                });
            }
            Err(message) => {
                ui.set_heading(tr("Could not start").into());
                ui.set_status(message.into());
                ui.set_state("error".into());
            }
        });
    });

    ui.show()?;
    place(&ui);
    slint::run_event_loop()?;
    Ok(())
}

/// Switches the profile on (if it is not already) and waits until it runs.
/// Returns the profile's name.
async fn start_profile(ui: &Weak<LaunchWindow>, id: &str) -> Result<String, String> {
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
    set(ui, Some(trf!("Starting {}", name)), None);
    if matches!(state.status, ProfileStatus::Running { .. }) {
        return Ok(name);
    }

    set(ui, None, Some(tr("Turning the profile on…")));
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
            ProfileStatus::Running { .. } => return Ok(name),
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

fn set(ui: &Weak<LaunchWindow>, heading: Option<String>, status: Option<String>) {
    let _ = ui.upgrade_in_event_loop(move |ui| {
        if let Some(h) = heading {
            ui.set_heading(h.into());
        }
        if let Some(s) = status {
            ui.set_status(s.into());
        }
    });
}

/// "Discord" for `…\Discord.lnk`, `/usr/share/applications/discord.desktop`, `…/Discord.exe`.
fn display_name(target: &str) -> String {
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

/// Bottom-right above the taskbar on Windows, like a notification; elsewhere the window
/// manager places it (Wayland does not let apps position windows anyway).
fn place(ui: &LaunchWindow) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SystemParametersInfoW};
        let mut area: RECT = unsafe { std::mem::zeroed() };
        // SAFETY: SPI_GETWORKAREA writes one RECT into the pointer we pass.
        let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut area as *mut _ as _, 0) };
        if ok != 0 {
            let size = ui.window().size();
            let margin = (16.0 * ui.window().scale_factor()) as i32;
            ui.window().set_position(slint::PhysicalPosition::new(
                area.right - size.width as i32 - margin,
                area.bottom - size.height as i32 - margin,
            ));
        }
    }
    #[cfg(not(windows))]
    let _ = ui;
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
