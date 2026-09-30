//! launchd integration for macOS: install, upgrade and remove the root service.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use dpimech_core::paths::{DataDir, default_service_data_dir};

const LABEL: &str = "io.github.dpimech.service";
const PLIST: &str = "/Library/LaunchDaemons/io.github.dpimech.service.plist";
/// Root-owned, like Program Files on Windows: never run the service from a user folder.
const INSTALL_DIR: &str = "/Library/PrivilegedHelperTools";
const LOG: &str = "/Library/Logs/DPIMech/service.log";

pub fn install(data: &DataDir) -> anyhow::Result<()> {
    require_root()?;
    // Unloading first releases the old binary; failure just means it was not installed.
    let _ = launchctl(&["bootout", &format!("system/{LABEL}")]);

    let exe = install_binary()?;
    crate::acl::harden_data_dir(&data.root)?;
    println!("data directory secured: {}", data.root.display());
    if let Some(dir) = Path::new(LOG).parent() {
        std::fs::create_dir_all(dir)?;
    }
    write_root_file(Path::new(PLIST), plist(&exe, &data.root).as_bytes(), 0o644)?;
    launchctl(&["bootstrap", "system", PLIST])?;
    println!("service installed and started from {}", exe.display());
    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    require_root()?;
    let _ = launchctl(&["bootout", &format!("system/{LABEL}")]);
    let _ = std::fs::remove_file(PLIST);
    let _ = std::fs::remove_file(Path::new(INSTALL_DIR).join("dpimech-service"));
    println!("service removed (profiles and engines in the data directory were kept)");
    Ok(())
}

/// KeepAlive restarts the service if it crashes (not after a clean stop); launchd stops the
/// whole process group with it, which takes the engines down too.
fn plist(exe: &Path, data_dir: &Path) -> String {
    let mut args = vec![exe.display().to_string(), "service".to_owned()];
    if data_dir != default_service_data_dir() {
        args.push("--data-dir".to_owned());
        args.push(data_dir.display().to_string());
    }
    let args: String = args
        .iter()
        .map(|a| format!("        <string>{}</string>\n", xml_escape(a)))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
{args}    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardErrorPath</key>
    <string>{LOG}</string>
    <key>StandardOutPath</key>
    <string>{LOG}</string>
</dict>
</plist>
"#
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn install_binary() -> anyhow::Result<PathBuf> {
    let current = std::env::current_exe()?;
    let target = Path::new(INSTALL_DIR).join("dpimech-service");
    if current == target {
        return Ok(target);
    }
    let bytes = std::fs::read(&current).context("reading the service binary")?;
    write_root_file(&target, &bytes, 0o755)?;
    Ok(target)
}

/// Temporary file + rename, ownership and mode set before the file becomes visible.
fn write_root_file(path: &Path, bytes: &[u8], mode: u32) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = path.parent().context("path has no parent")?;
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    let _ = std::fs::remove_file(&tmp);
    std::fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    std::os::unix::fs::lchown(&tmp, Some(0), Some(0))?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode))?;
    std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn require_root() -> anyhow::Result<()> {
    // SAFETY: geteuid has no preconditions.
    if unsafe { libc::geteuid() } != 0 {
        bail!("run this as root (sudo dpimech-service install)");
    }
    Ok(())
}

fn launchctl(args: &[&str]) -> anyhow::Result<()> {
    let out = Command::new("launchctl")
        .args(args)
        .output()
        .context("running launchctl")?;
    if !out.status.success() {
        bail!(
            "launchctl {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_runs_the_installed_binary_as_a_service() {
        let text = plist(
            Path::new("/Library/PrivilegedHelperTools/dpimech-service"),
            Path::new("/tmp/a&b"),
        );
        assert!(text.contains("<string>/Library/PrivilegedHelperTools/dpimech-service</string>"));
        assert!(text.contains("<string>service</string>"));
        assert!(text.contains("<string>/tmp/a&amp;b</string>"));
        assert!(text.contains("<key>KeepAlive</key>"));
    }
}
