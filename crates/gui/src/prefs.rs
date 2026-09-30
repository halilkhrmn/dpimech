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

/// Starts `dpimech-service.exe install` elevated (one UAC prompt). The service binary ships
/// next to the GUI.
#[cfg(windows)]
pub fn install_service_elevated() -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name("dpimech-service.exe");
    if !exe.is_file() {
        return Err(format!("{} not found", exe.display()));
    }
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (verb, file, params) = (
        wide("runas"),
        wide(&exe.display().to_string()),
        wide("install"),
    );
    // SAFETY: all strings are NUL-terminated UTF-16 that outlive the call.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_HIDE,
        )
    };
    // Values above 32 mean success; the user may still decline the UAC prompt.
    if result as isize > 32 {
        Ok(())
    } else {
        Err("Windows did not start the installer (was the prompt declined?)".into())
    }
}

/// Runs `dpimech-service install` through polkit (`pkexec`), which asks for the admin
/// password once. Not waited for, like the Windows prompt, so the window stays responsive;
/// the GUI notices the service when it connects.
/// With a deb/rpm the service is already installed and only needs to be started.
#[cfg(all(unix, not(target_os = "macos")))]
const PACKAGED_SERVICE: &str = "/usr/lib/dpimech/dpimech-service";

#[cfg(all(unix, not(target_os = "macos")))]
pub fn install_service_elevated() -> Result<(), String> {
    let (program, args): (PathBuf, Vec<&str>) = if std::path::Path::new(PACKAGED_SERVICE).is_file()
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
        .map(drop)
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
pub fn install_service_elevated() -> Result<(), String> {
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
        .map(drop)
        .map_err(|e| {
            format!(
                "could not ask for administrator rights ({e}); run: sudo '{}' install",
                exe.display()
            )
        })
}
