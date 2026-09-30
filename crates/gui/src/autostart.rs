//! "Start with Windows" for the GUI via the per-user Run key (no elevation needed).
//! The service starts on its own; this only brings back the tray icon after login.
//! Linux uses an XDG autostart entry. TODO(phase 7): LaunchAgent on macOS.

pub const MINIMIZED_FLAG: &str = "--minimized";

#[derive(Debug, Clone, Copy, Default)]
pub struct Autostart {
    pub enabled: bool,
    pub minimized: bool,
}

#[cfg(windows)]
mod imp {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

    use super::{Autostart, MINIMIZED_FLAG};

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE: &str = "dpimech";

    pub fn get() -> Autostart {
        let command: Option<String> = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_READ)
            .and_then(|k| k.get_value(VALUE))
            .ok();
        match command {
            Some(cmd) => Autostart {
                enabled: true,
                minimized: cmd.contains(MINIMIZED_FLAG),
            },
            None => Autostart::default(),
        }
    }

    /// 0.1.x registered itself as "dpimngr"; that entry points at the old exe.
    pub fn migrate_legacy() {
        let Ok(key) =
            RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)
        else {
            return;
        };
        let Ok(old) = key.get_value::<String, _>("dpimngr") else {
            return;
        };
        let _ = key.delete_value("dpimngr");
        let _ = set(Autostart {
            enabled: true,
            minimized: old.contains(MINIMIZED_FLAG),
        });
    }

    pub fn set(settings: Autostart) -> std::io::Result<()> {
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_WRITE)?;
        if !settings.enabled {
            return match key.delete_value(VALUE) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            };
        }
        let exe = std::env::current_exe()?;
        let mut command = format!("\"{}\"", exe.display());
        if settings.minimized {
            command.push(' ');
            command.push_str(MINIMIZED_FLAG);
        }
        key.set_value(VALUE, &command)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use std::path::PathBuf;

    use super::{Autostart, MINIMIZED_FLAG};

    /// `$XDG_CONFIG_HOME/autostart/dpimech.desktop` (freedesktop Desktop Application Autostart).
    fn entry_path() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|dir| dir.join("autostart").join("dpimech.desktop"))
    }

    /// 0.1.x wrote `dpimngr.desktop`, which starts the old binary.
    pub fn migrate_legacy() {
        let Some(new) = entry_path() else { return };
        let old = new.with_file_name("dpimngr.desktop");
        let Ok(text) = std::fs::read_to_string(&old) else {
            return;
        };
        let _ = std::fs::remove_file(&old);
        if !text.lines().any(|l| l.trim() == "Hidden=true") {
            let _ = set(Autostart {
                enabled: true,
                minimized: text.contains(MINIMIZED_FLAG),
            });
        }
    }

    pub fn get() -> Autostart {
        let Some(text) = entry_path().and_then(|p| std::fs::read_to_string(p).ok()) else {
            return Autostart::default();
        };
        let exec = text
            .lines()
            .find_map(|l| l.strip_prefix("Exec="))
            .unwrap_or_default();
        Autostart {
            enabled: !text.lines().any(|l| l.trim() == "Hidden=true"),
            minimized: exec.contains(MINIMIZED_FLAG),
        }
    }

    pub fn set(settings: Autostart) -> std::io::Result<()> {
        let path = entry_path().ok_or_else(|| std::io::Error::other("no home directory"))?;
        if !settings.enabled {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            };
        }
        // An AppImage runs from a new mount point each time; $APPIMAGE is the file itself.
        let exe = match std::env::var_os("APPIMAGE") {
            Some(image) => std::path::PathBuf::from(image),
            None => std::env::current_exe()?,
        };
        let mut exec = quote(&exe.to_string_lossy());
        if settings.minimized {
            exec.push(' ');
            exec.push_str(MINIMIZED_FLAG);
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, entry(&exec))
    }

    fn entry(exec: &str) -> String {
        format!(
            "[Desktop Entry]\nType=Application\nName=DPIMech\nComment=DPI bypass manager\nExec={exec}\nIcon=dpimech\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
        )
    }

    /// Desktop Entry `Exec` quoting: double quotes, with `"`, `` ` ``, `$` and `\` escaped by
    /// a backslash, which the file's string escaping then doubles again; `%` starts a field
    /// code, so it is doubled too.
    pub(super) fn quote(arg: &str) -> String {
        let mut out = String::from("\"");
        for c in arg.chars() {
            match c {
                '"' | '`' | '$' => {
                    out.push_str("\\\\");
                    out.push(c);
                }
                '\\' => out.push_str("\\\\\\\\"),
                '%' => out.push_str("%%"),
                _ => out.push(c),
            }
        }
        out.push('"');
        out
    }
}

/// macOS: a per-user LaunchAgent (`~/Library/LaunchAgents`), no admin rights needed.
#[cfg(target_os = "macos")]
mod imp {
    use std::path::PathBuf;

    use super::{Autostart, MINIMIZED_FLAG};

    const LABEL: &str = "io.github.dpimech.gui";

    fn agent_path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library/LaunchAgents")
                .join(format!("{LABEL}.plist"))
        })
    }

    pub fn migrate_legacy() {}

    pub fn get() -> Autostart {
        match agent_path().and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(text) => Autostart {
                enabled: true,
                minimized: text.contains(MINIMIZED_FLAG),
            },
            None => Autostart::default(),
        }
    }

    pub fn set(settings: Autostart) -> std::io::Result<()> {
        let path = agent_path().ok_or_else(|| std::io::Error::other("no home directory"))?;
        if !settings.enabled {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            };
        }
        let exe = std::env::current_exe()?;
        let mut args = format!("        <string>{}</string>\n", xml(&exe.to_string_lossy()));
        if settings.minimized {
            args.push_str(&format!("        <string>{MINIMIZED_FLAG}</string>\n"));
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(
            &path,
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n    <key>Label</key>\n    <string>{LABEL}</string>\n    <key>ProgramArguments</key>\n    <array>\n{args}    </array>\n    <key>RunAtLoad</key>\n    <true/>\n</dict>\n</plist>\n"
            ),
        )
    }

    fn xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

pub use imp::{get, migrate_legacy, set};

#[cfg(all(test, unix, not(target_os = "macos")))]
mod tests {
    #[test]
    fn exec_quoting_follows_the_desktop_entry_spec() {
        assert_eq!(
            super::imp::quote("/opt/my apps/dpi$mngr 100%"),
            r#""/opt/my apps/dpi\\$mngr 100%%""#
        );
    }
}
