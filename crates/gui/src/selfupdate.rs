//! Updating DPIMech from inside the app. The service downloads and verifies the new release
//! (it has the network code and an admin-only folder) and, on Windows, runs the installer
//! silently. An AppImage belongs to the user, so the GUI swaps that file itself. Copies from
//! a package manager (deb, rpm, COPR) are left to it.

use std::cell::RefCell;

use dpimech_core::ipc::UpdateFormat;

use crate::AppWindow;
use crate::i18n::tr;

static READY_VERSION: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

thread_local! {
    /// The downloaded update: version and file.
    static READY: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// How this copy of DPIMech can update itself, if it can.
pub fn format() -> Option<UpdateFormat> {
    if cfg!(windows) {
        return Some(UpdateFormat::WindowsInstaller);
    }
    if cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_some() {
        return Some(UpdateFormat::AppImage);
    }
    None
}

/// What to tell users whose copy cannot update itself.
pub fn manual_hint() -> String {
    if cfg!(target_os = "linux") {
        tr("Update it with your package manager (Fedora: sudo dnf upgrade), or download it.")
    } else {
        tr("Download it from the releases page.")
    }
}

/// The service has downloaded and checked `version`: offer the restart.
pub fn ready(ui: &AppWindow, version: String, path: String) {
    let version = version.trim_start_matches('v').to_owned();
    ui.set_app_update_ready(true);
    ui.set_app_update_version(version.clone().into());
    ui.set_app_update_text(
        trf!(
            "DPIMech {} is downloaded. Restart DPIMech to update.",
            version
        )
        .into(),
    );
    crate::notify::update_ready(&version);
    *READY_VERSION.lock().unwrap() = Some(version.clone());
    READY.set(Some((version, path)));
}

/// The update is already downloaded (e.g. the daily check found the same version again).
/// Thread-safe: the bridge thread asks too, so it reads a copy kept outside the UI thread.
pub fn ready_version_matches(version: &str) -> bool {
    READY_VERSION.lock().unwrap().as_deref() == Some(version)
}

#[cfg_attr(not(windows), allow(dead_code))] // the Windows relaunch helper needs it
pub fn ready_version() -> Option<String> {
    READY.with_borrow(|r| r.as_ref().map(|(v, _)| v.clone()))
}

/// AppImage: puts the downloaded file in place of the running one and starts it. The running
/// copy keeps working until it exits: its mount still refers to the old file.
pub fn swap_appimage() -> Result<(), String> {
    let (_, path) = READY
        .with_borrow(|r| r.clone())
        .ok_or_else(|| tr("No update has been downloaded yet."))?;
    let target = std::path::PathBuf::from(
        std::env::var_os("APPIMAGE").ok_or_else(|| tr("This copy is not an AppImage."))?,
    );
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = target.with_file_name(format!(".{name}.update"));
    let copy = || -> std::io::Result<()> {
        std::fs::copy(&path, &tmp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&tmp, &target)
    };
    if let Err(e) = copy() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    // Started only once this process has exited: a second DPIMech would otherwise just show
    // this (closing) window and quit.
    std::process::Command::new("sh")
        .args([
            "-c",
            r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec "$0""#,
        ])
        .arg(&target)
        .arg(std::process::id().to_string())
        .spawn()
        .map(drop)
        .map_err(|e| e.to_string())
}

/// Windows: the installer (started by the service) closes this window, replaces the files and
/// restarts the service, but it cannot start the window again in the user's session. A hidden
/// PowerShell waits for the new version to be in place and does that. Values travel in
/// environment variables, never inside the script.
#[cfg(windows)]
pub fn relaunch_after_install() -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = r#"$deadline = (Get-Date).AddMinutes(10)
Start-Sleep -Seconds 3
while ((Get-Date) -lt $deadline) {
  $setup = Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'dpimech-setup*' }
  $version = (Get-Item $env:DPIMECH_EXE -ErrorAction SilentlyContinue).VersionInfo.ProductVersion
  if (-not $setup -and $version -and $version.StartsWith($env:DPIMECH_VERSION)) { break }
  Start-Sleep -Seconds 2
}
Start-Sleep -Seconds 2
Start-Process -FilePath $env:DPIMECH_EXE"#;

    let version = ready_version()
        .ok_or_else(|| tr("No update has been downloaded yet."))?
        .trim_start_matches('v')
        .to_owned();
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            SCRIPT,
        ])
        .env("DPIMECH_EXE", exe)
        .env("DPIMECH_VERSION", version)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(drop)
        .map_err(|e| e.to_string())
}

/// "Restart and update" pressed: AppImage swaps here; Windows asks the service (the bridge
/// calls `relaunch_after_install` once the installer runs).
pub fn restart_and_update(
    ui: &AppWindow,
    tx: &tokio::sync::mpsc::UnboundedSender<crate::bridge::Command>,
) {
    match format() {
        Some(UpdateFormat::AppImage) => match swap_appimage() {
            Ok(()) => {
                let _ = slint::quit_event_loop();
            }
            Err(e) => ui.set_app_update_text(trf!("Could not update: {}", e).into()),
        },
        Some(UpdateFormat::WindowsInstaller) => {
            ui.set_app_update_text(tr("Installing the update…").into());
            let _ = tx.send(crate::bridge::Command::InstallUpdate);
        }
        None => {}
    }
}
