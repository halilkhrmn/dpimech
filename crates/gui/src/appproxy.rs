//! "Open with proxy" profiles: the engine is only a local SOCKS5 port, and the
//! GUI opens the profile's app with `--proxy-server` pointing at it. Chromium and Electron apps
//! (Discord, Chrome, Edge, Brave…) honour that switch, so no packet driver is involved.
//!
//! Profiles are shared by every user of the computer while the app runs as whoever switches the
//! profile on. So the app is only opened from places another user cannot write to: otherwise one
//! user could make another run a program of their choosing.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dpimech_core::model::{ProfileState, ProfileStatus, Routing};

use crate::i18n::tr;

/// Profiles switched on from this window or the tray, waiting to run before their app opens.
static PENDING: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());
/// Longer than any engine takes to start (ProxiFyre is not involved here).
const PENDING_FOR: Duration = Duration::from_secs(60);

pub fn proxy_flag(port: u16) -> String {
    format!("--proxy-server=socks5://127.0.0.1:{port}")
}

/// Remembers that `id` was just switched on; its app opens once the engine runs.
/// Profiles started by the service on its own (at boot) do not open their app.
pub fn open_when_running(id: &str) {
    let mut pending = PENDING.lock().unwrap();
    pending.retain(|(p, _)| p != id);
    pending.push((id.to_owned(), Instant::now()));
}

/// Called with every profile list from the service.
pub fn on_profiles(profiles: &[ProfileState]) {
    let ready = take_ready(&mut PENDING.lock().unwrap(), profiles, Instant::now());
    for (app, port) in ready {
        // Resolving the app touches the disk; keep it off the UI thread.
        std::thread::spawn(move || {
            if let Err(e) = open(&app, port) {
                eprintln!("opening {app}: {e}");
                crate::notify::app_open_failed(&crate::launcher::display_name(&app), &e);
            }
        });
    }
}

/// Removes the pending profiles that are now running (returning the apps to open) or will not
/// run after all.
fn take_ready(
    pending: &mut Vec<(String, Instant)>,
    profiles: &[ProfileState],
    now: Instant,
) -> Vec<(String, u16)> {
    let mut ready = Vec::new();
    pending.retain(|(id, since)| {
        let Some(state) = profiles.iter().find(|p| &p.profile.id == id) else {
            return false;
        };
        match (&state.status, &state.profile.routing) {
            (ProfileStatus::Running { .. }, Routing::AppProxy { app, port }) => {
                ready.push((app.clone(), *port));
                false
            }
            (ProfileStatus::Running { .. } | ProfileStatus::Error { .. }, _) => false,
            // Stopped can still be the state from before the start request arrived.
            _ => now.duration_since(*since) < PENDING_FOR,
        }
    });
    ready
}

/// Opens `app` with its traffic going to the engine on `port`.
pub fn open(app: &str, port: u16) -> Result<(), String> {
    platform::open(app, &proxy_flag(port))
}

/// Whether `app` looks like a Chromium or Electron app; `None` when it cannot be told
/// (a Flatpak, a command that is not installed…).
pub fn looks_chromium(app: &str) -> Option<bool> {
    platform::looks_chromium(app)
}

/// Chromium keeps its `.pak` resources next to the executable (Electron apps, Chrome on Linux)
/// or in a version folder below it (Chrome and Edge on Windows).
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn has_chromium_files(dir: &Path) -> Option<bool> {
    let has_pak = |d: &Path| {
        ["resources.pak", "chrome_100_percent.pak"]
            .iter()
            .any(|f| d.join(f).is_file())
    };
    let entries = std::fs::read_dir(dir).ok()?;
    if has_pak(dir) {
        return Some(true);
    }
    Some(
        entries
            .flatten()
            .any(|e| e.file_type().is_ok_and(|t| t.is_dir()) && has_pak(&e.path())),
    )
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    pub fn open(app: &str, flag: &str) -> Result<(), String> {
        let mut cmd = match squirrel(Path::new(app)) {
            // Discord, Slack, Teams classic…: Update.exe starts the newest installed version and
            // keeps working after the app updated itself into a new folder.
            Some((update, exe)) => {
                let mut cmd = Command::new(trusted(&update)?);
                cmd.args(["--processStart", &exe, "--process-start-args", flag]);
                cmd
            }
            None => {
                let mut cmd = Command::new(trusted(Path::new(app))?);
                cmd.arg(flag);
                cmd
            }
        };
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(drop)
            .map_err(|e| e.to_string())
    }

    pub fn looks_chromium(app: &str) -> Option<bool> {
        let path = Path::new(app);
        let exe_dir = match squirrel(path) {
            Some((update, exe)) => newest_version_dir(update.parent()?, &exe)?,
            None => path.parent()?.to_path_buf(),
        };
        has_chromium_files(&exe_dir)
    }

    /// `(Update.exe, "Discord.exe")` for a Squirrel install: either `…\Discord\Update.exe` or
    /// `…\Discord\app-1.0.9205\Discord.exe`.
    pub(super) fn squirrel(path: &Path) -> Option<(PathBuf, String)> {
        let name = path.file_name()?.to_string_lossy().into_owned();
        if name.eq_ignore_ascii_case("Update.exe") {
            let root = path.parent()?;
            // The app is named after its folder: Discord\app-…\Discord.exe.
            let exe = format!("{}.exe", root.file_name()?.to_string_lossy());
            return newest_version_dir(root, &exe).map(|_| (path.to_path_buf(), exe));
        }
        let version_dir = path.parent()?;
        let is_version = version_dir
            .file_name()?
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with("app-");
        let update = version_dir.parent()?.join("Update.exe");
        (is_version && update.is_file()).then_some((update, name))
    }

    pub(super) fn newest_version_dir(root: &Path, exe: &str) -> Option<PathBuf> {
        let version = |p: &Path| -> Vec<u32> {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
                .trim_start_matches("app-")
                .split('.')
                .filter_map(|n| n.parse().ok())
                .collect()
        };
        std::fs::read_dir(root)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("app-"))
                    && p.join(exe).is_file()
            })
            .max_by_key(|p| version(p))
    }

    /// Program Files, Windows and this user's own app folders; never another user's profile
    /// or a folder anyone can write to, such as `C:\Temp`.
    fn trusted(path: &Path) -> Result<PathBuf, String> {
        let text = path.to_string_lossy().to_ascii_lowercase();
        let allowed = [
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
            "SystemRoot",
            "LOCALAPPDATA",
            "APPDATA",
        ]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .map(|d| format!("{}\\", d.trim_end_matches('\\').to_ascii_lowercase()));
        let inside = !text.contains("..") && allowed.into_iter().any(|d| text.starts_with(&d));
        if !inside {
            return Err(tr(
                "DPIMech only opens apps from Program Files or your own user folder.",
            ));
        }
        if !path.is_file() {
            return Err(tr(
                "the app is no longer there; choose it again in the profile",
            ));
        }
        Ok(path.to_path_buf())
    }
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    use std::process::Command;
    #[cfg(not(target_os = "macos"))]
    use std::{path::PathBuf, process::Stdio};

    /// Owned by root or by this user, and nobody else may change it or the folder it is in.
    /// Checked on the real file, so a link in a safe folder cannot point somewhere unsafe.
    pub(super) fn trusted(path: &Path) -> Result<(), String> {
        // SAFETY: getuid cannot fail.
        let me = unsafe { libc::getuid() };
        let safe = |p: &Path| {
            std::fs::metadata(p)
                .is_ok_and(|m| (m.uid() == 0 || m.uid() == me) && m.mode() & 0o022 == 0)
        };
        let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let parent = real.parent().unwrap_or(Path::new("/"));
        if safe(&real) && safe(parent) {
            Ok(())
        } else {
            Err(tr(
                "DPIMech only opens apps that belong to the system or to you and that other users cannot change.",
            ))
        }
    }

    /// A bare command is looked up in this user's PATH, like a terminal would.
    #[cfg(not(target_os = "macos"))]
    fn resolve(program: &str) -> Option<PathBuf> {
        if program.contains('/') {
            return Some(PathBuf::from(program));
        }
        std::env::split_paths(&std::env::var_os("PATH")?)
            .map(|d| d.join(program))
            .find(|p| p.is_file())
    }

    /// The program and its arguments; for a `.desktop` entry the words of its `Exec` line.
    #[cfg(not(target_os = "macos"))]
    fn command_line(app: &str) -> Result<Vec<String>, String> {
        if !app.ends_with(".desktop") {
            return Ok(vec![app.to_owned()]);
        }
        trusted(Path::new(app))?;
        let text = std::fs::read_to_string(app).map_err(|e| e.to_string())?;
        let exec = crate::shortcut::linux::desktop_value(&text, "Exec")
            .ok_or_else(|| tr("the menu entry has no command"))?;
        let words: Vec<String> = dpimech_core::args::split_args(&exec)
            .into_iter()
            .filter(|w| !(w.starts_with('%') && w.len() == 2))
            .collect();
        if words.is_empty() {
            return Err(tr("the menu entry has no command"));
        }
        Ok(words)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn open(app: &str, flag: &str) -> Result<(), String> {
        let words = command_line(app)?;
        let program = resolve(&words[0])
            .ok_or_else(|| tr("the app is no longer there; choose it again in the profile"))?;
        trusted(&program)?;
        Command::new(&program)
            .args(&words[1..])
            .arg(flag)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(drop)
            .map_err(|e| e.to_string())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn looks_chromium(app: &str) -> Option<bool> {
        let words = command_line(app).ok()?;
        let program = std::fs::canonicalize(resolve(&words[0])?).ok()?;
        // Sandboxed or wrapped apps: what really runs is not visible from here.
        let name = program.file_name()?.to_string_lossy().into_owned();
        if ["flatpak", "snap", "env", "sh", "bash"].contains(&name.as_str()) {
            return None;
        }
        has_chromium_files(program.parent()?)
    }

    #[cfg(target_os = "macos")]
    pub fn open(app: &str, flag: &str) -> Result<(), String> {
        let bundle = Path::new(app);
        if !bundle.exists() {
            return Err(tr(
                "the app is no longer there; choose it again in the profile",
            ));
        }
        trusted(bundle)?;
        // `open` passes --args only when it starts the app; a running app keeps its settings.
        let status = Command::new("open")
            .arg("-a")
            .arg(bundle)
            .args(["--args", flag])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(status.to_string())
        }
    }

    /// Chromium-based bundles ship "<Name> Framework.framework" (Electron Framework,
    /// Google Chrome Framework…).
    #[cfg(target_os = "macos")]
    pub fn looks_chromium(app: &str) -> Option<bool> {
        if !Path::new(app).join("Contents").is_dir() {
            return None;
        }
        let frameworks = Path::new(app).join("Contents/Frameworks");
        Some(std::fs::read_dir(frameworks).is_ok_and(|d| {
            d.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .ends_with(" Framework.framework")
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpimech_core::model::{Profile, Reliability};

    fn state(id: &str, routing: Routing, status: ProfileStatus) -> ProfileState {
        ProfileState {
            profile: Profile {
                id: id.into(),
                name: id.into(),
                engine: dpimech_core::model::EngineKind::ByeDpi,
                args: String::new(),
                routing,
                autostart: false,
                reliability: Reliability::default(),
                check_sites: Vec::new(),
            },
            status,
        }
    }

    #[test]
    fn the_app_opens_once_the_profile_runs() {
        let app = || Routing::AppProxy {
            app: "/usr/bin/discord".into(),
            port: 1080,
        };
        let running = ProfileStatus::Running {
            since_unix: 0,
            restarts: 0,
            health: None,
        };
        let t0 = Instant::now();
        let mut pending = vec![("a".to_owned(), t0), ("b".to_owned(), t0)];
        let stopped = [
            state("a", app(), ProfileStatus::Stopped),
            state(
                "b",
                Routing::LocalProxy { port: 1081 },
                ProfileStatus::Stopped,
            ),
        ];
        // Not running yet: both wait.
        assert!(take_ready(&mut pending, &stopped, t0).is_empty());
        assert_eq!(pending.len(), 2);
        let started = [
            state("a", app(), running.clone()),
            state("b", Routing::LocalProxy { port: 1081 }, running),
        ];
        assert_eq!(
            take_ready(&mut pending, &started, t0),
            vec![("/usr/bin/discord".to_owned(), 1080)]
        );
        // Opened once; a plain proxy profile opens nothing and is forgotten too.
        assert!(pending.is_empty());

        // A start that never happens is dropped after a while.
        let mut pending = vec![("a".to_owned(), t0)];
        let later = t0 + PENDING_FOR + Duration::from_secs(1);
        assert!(take_ready(&mut pending, &stopped, later).is_empty());
        assert!(pending.is_empty());
    }

    #[test]
    fn flag_points_at_the_local_engine() {
        assert_eq!(proxy_flag(1080), "--proxy-server=socks5://127.0.0.1:1080");
    }

    #[test]
    fn chromium_files_next_to_or_below_the_exe() {
        let dir = std::env::temp_dir().join(format!("dpimech-appproxy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("120.0.1")).unwrap();
        assert_eq!(has_chromium_files(&dir), Some(false));
        std::fs::write(dir.join("120.0.1/resources.pak"), b"").unwrap();
        assert_eq!(has_chromium_files(&dir), Some(true));
        assert_eq!(has_chromium_files(&dir.join("missing")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn squirrel_apps_start_through_update_exe() {
        let root = std::env::temp_dir()
            .join(format!("dpimech-squirrel-{}", std::process::id()))
            .join("Discord");
        let _ = std::fs::remove_dir_all(&root);
        for v in ["app-1.0.9", "app-1.0.10"] {
            std::fs::create_dir_all(root.join(v)).unwrap();
            std::fs::write(root.join(v).join("Discord.exe"), b"").unwrap();
        }
        std::fs::write(root.join("Update.exe"), b"").unwrap();
        let update = root.join("Update.exe");
        // Picked by its Start Menu shortcut (Update.exe) or as the running Discord.exe.
        assert_eq!(
            platform::squirrel(&update),
            Some((update.clone(), "Discord.exe".to_owned()))
        );
        assert_eq!(
            platform::squirrel(&root.join("app-1.0.9").join("Discord.exe")),
            Some((update.clone(), "Discord.exe".to_owned()))
        );
        // 1.0.10 is newer than 1.0.9 although it sorts first as text.
        assert_eq!(
            platform::newest_version_dir(&root, "Discord.exe"),
            Some(root.join("app-1.0.10"))
        );
        assert_eq!(platform::squirrel(&root.join("other.exe")), None);
        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn apps_others_can_change_are_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("dpimech-trust-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let app = dir.join("app");
        std::fs::write(&app, b"").unwrap();
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(platform::trusted(&app).is_ok());
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(platform::trusted(&app).is_err());
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o755)).unwrap();
        // /tmp itself: anyone may replace files in it.
        assert!(platform::trusted(Path::new("/tmp/x")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
