//! Per-user GUI preferences (`%APPDATA%\dpimech\gui.toml`). Service settings live elsewhere.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    /// Simplified interface for people who just want it to work.
    #[serde(default)]
    pub easy_mode: bool,
    /// The first-run question was answered.
    #[serde(default)]
    pub onboarded: bool,
    /// Also write the log to daily files (see logfile.rs).
    #[serde(default = "default_true")]
    pub log_to_file: bool,
    /// Folder for the log files; `None` = the default per-user folder.
    #[serde(default)]
    pub log_dir: Option<PathBuf>,
    /// "tr", "ru", "en"; empty = follow the system language.
    #[serde(default)]
    pub language: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            easy_mode: false,
            onboarded: false,
            log_to_file: true,
            log_dir: None,
            language: String::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

impl Prefs {
    /// Where log files go right now, or `None` when writing is off.
    pub fn log_dir(&self) -> Option<PathBuf> {
        if !self.log_to_file {
            return None;
        }
        self.log_dir.clone().or_else(crate::logfile::default_dir)
    }
}

/// `%APPDATA%` on Windows, `~/Library/Application Support` on macOS, `$XDG_CONFIG_HOME` or
/// `~/.config` elsewhere.
fn config_base() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support"));
    }
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

fn path() -> Option<PathBuf> {
    Some(config_base()?.join("dpimech").join("gui.toml"))
}

/// Carries the preferences of 0.1.x (saved under the working name "dpimngr") over once, so
/// an upgrade does not ask the first-run question again.
pub fn migrate_legacy() {
    let (Some(base), Some(new)) = (config_base(), path()) else {
        return;
    };
    let old = base.join("dpimngr").join("gui.toml");
    if old.is_file() && !new.exists() {
        if let Some(dir) = new.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::copy(&old, &new);
    }
}

pub fn load() -> Prefs {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(prefs: &Prefs) {
    let Some(path) = path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = toml::to_string_pretty(prefs) {
        let _ = std::fs::write(path, text);
    }
}

/// Starts `dpimech-service.exe install` elevated (one UAC prompt); the service binary ships
/// next to the GUI. `done` runs on another thread once the installer has finished.
#[cfg(windows)]
pub fn install_service_elevated(
    done: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, GetLastError};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, INFINITE, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name("dpimech-service.exe");
    if !exe.is_file() {
        return Err(format!("{} not found", exe.display()));
    }
    let started = std::time::SystemTime::now();
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (verb, file, params) = (
        wide("runas"),
        wide(&exe.display().to_string()),
        wide("install"),
    );
    // SAFETY: the struct is zero-initialised with its size set; all strings are NUL-terminated
    // UTF-16 that outlive the call.
    let process = unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = params.as_ptr();
        info.nShow = SW_HIDE;
        if ShellExecuteExW(&mut info) == 0 {
            return Err(if GetLastError() == ERROR_CANCELLED {
                "the administrator prompt was declined".into()
            } else {
                std::io::Error::last_os_error().to_string()
            });
        }
        info.hProcess as usize
    };
    std::thread::spawn(move || {
        let mut code = 1u32;
        // SAFETY: the handle came from ShellExecuteExW (NOCLOSEPROCESS) and is closed here.
        unsafe {
            WaitForSingleObject(process as _, INFINITE);
            GetExitCodeProcess(process as _, &mut code);
            CloseHandle(process as _);
        }
        done(if code == 0 {
            Ok(())
        } else {
            Err(install_error(started))
        });
    });
    Ok(())
}

/// Why `dpimech-service install` failed, from the log it writes (see the service's main.rs);
/// a log older than this attempt belongs to an earlier one and is ignored.
fn install_error(started: std::time::SystemTime) -> String {
    let log = dpimech_core::paths::default_service_data_dir()
        .join("logs")
        .join("install.log");
    let fresh = std::fs::metadata(&log)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t >= started);
    fresh
        .then(|| std::fs::read_to_string(&log).ok())
        .flatten()
        .and_then(|t| {
            t.lines()
                .rev()
                .find(|l| l.starts_with("error: "))
                .map(str::to_owned)
        })
        .map(|l| l.trim_start_matches("error: ").to_owned())
        .unwrap_or_else(|| "the installer stopped with an error".into())
}

/// Waits for a started installer (pkexec, osascript) on another thread and reports to `done`.
#[cfg(unix)]
fn wait_for(
    mut child: std::process::Child,
    done: impl FnOnce(Result<(), String>) + Send + 'static,
) {
    let started = std::time::SystemTime::now();
    std::thread::spawn(move || {
        let result = match child.wait() {
            Ok(status) if status.success() => Ok(()),
            // pkexec: 126 = dialog dismissed, 127 = not authorised; osascript: 1 on cancel.
            Ok(status) if matches!(status.code(), Some(126 | 127)) => {
                Err("the administrator prompt was declined".into())
            }
            Ok(_) => Err(install_error(started)),
            Err(e) => Err(e.to_string()),
        };
        done(result);
    });
}

/// Runs `dpimech-service install` through polkit (`pkexec`), which asks for the admin
/// password once. `done` runs on another thread when it has finished; the GUI also notices
/// the service by itself when it connects.
/// With a deb/rpm the service is already installed and only needs to be started
/// (/usr/lib: deb and the release rpm; /usr/libexec: the Fedora COPR package).
#[cfg(all(unix, not(target_os = "macos")))]
const PACKAGED_SERVICE: [&str; 2] = [
    "/usr/lib/dpimech/dpimech-service",
    "/usr/libexec/dpimech/dpimech-service",
];

#[cfg(all(unix, not(target_os = "macos")))]
pub fn install_service_elevated(
    done: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Result<(), String> {
    let (program, args): (PathBuf, Vec<&str>) = if PACKAGED_SERVICE
        .iter()
        .any(|p| std::path::Path::new(p).is_file())
    {
        (
            "systemctl".into(),
            vec!["enable", "--now", "dpimech.service"],
        )
    } else {
        (service_binary_for_root()?, vec!["install"])
    };
    std::process::Command::new("pkexec")
        .arg(&program)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .spawn()
        .map(|child| wait_for(child, done))
        .map_err(|e| {
            format!(
                "could not start pkexec ({e}); run: sudo {} {}",
                program.display(),
                args.join(" ")
            )
        })
}

/// The service binary next to the GUI. Inside an AppImage that is a FUSE mount only this
/// user can read (not root), so it is first copied to the user's private runtime directory;
/// `install` then copies itself to /usr/local/lib/dpimech.
#[cfg(all(unix, not(target_os = "macos")))]
fn service_binary_for_root() -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;

    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name("dpimech-service");
    if !exe.is_file() {
        return Err(format!("{} not found", exe.display()));
    }
    if std::env::var_os("APPIMAGE").is_none() {
        return Ok(exe);
    }
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("dpimech");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let copy = dir.join("dpimech-service");
    std::fs::copy(&exe, &copy).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;
    Ok(copy)
}

/// macOS: the standard administrator password prompt through AppleScript; the service
/// binary ships next to the GUI inside DPIMech.app.
#[cfg(target_os = "macos")]
pub fn install_service_elevated(
    done: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name("dpimech-service");
    if !exe.is_file() {
        return Err(format!("{} not found", exe.display()));
    }
    // Single quotes for the shell, escaped for AppleScript's double-quoted string.
    let shell = format!(
        "'{}' install",
        exe.display().to_string().replace('\'', "'\\''")
    );
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        shell.replace('\\', "\\\\").replace('"', "\\\"")
    );
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .stdin(std::process::Stdio::null())
        .spawn()
        .map(|child| wait_for(child, done))
        .map_err(|e| {
            format!(
                "could not ask for administrator rights ({e}); run: sudo '{}' install",
                exe.display()
            )
        })
}
