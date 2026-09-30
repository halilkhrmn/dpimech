//! systemd integration for Linux: install, upgrade and remove the root service.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use dpimech_core::paths::{DataDir, default_service_data_dir};

const UNIT_NAME: &str = "dpimech.service";
const UNIT_PATH: &str = "/etc/systemd/system/dpimech.service";
/// Root-owned like Program Files on Windows: the service must never run from a folder a
/// user can write to.
const INSTALL_DIR: &str = "/usr/local/lib/dpimech";

/// Installs or upgrades the service. Running it again replaces the binary and restarts.
pub fn install(data: &DataDir) -> anyhow::Result<()> {
    require_root()?;
    // Stopping first releases the old binary; failure just means it was not installed.
    let _ = systemctl(&["stop", UNIT_NAME]);
    remove_legacy_service();
    crate::migrate::move_data_dir(data);

    let exe = install_binary()?;
    crate::acl::harden_data_dir(&data.root)?;
    println!("data directory secured: {}", data.root.display());

    write_root_file(
        Path::new(UNIT_PATH),
        unit_file(&exe, &data.root).as_bytes(),
        0o644,
    )?;
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", UNIT_NAME])?;
    systemctl(&["restart", UNIT_NAME])?;
    println!("service installed and started from {}", exe.display());
    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    require_root()?;
    let _ = systemctl(&["disable", "--now", UNIT_NAME]);
    match std::fs::remove_file(UNIT_PATH) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context(UNIT_PATH),
    }
    systemctl(&["daemon-reload"])?;
    let _ = std::fs::remove_file(Path::new(INSTALL_DIR).join("dpimech-service"));
    let _ = std::fs::remove_dir(INSTALL_DIR);
    println!("service removed (profiles and engines in the data directory were kept)");
    Ok(())
}

/// 0.1.x installed `dpimngr.service` from /usr/local/lib/dpimngr. Stopping it takes its
/// engines down (control group), which frees the old data directory for the move.
fn remove_legacy_service() {
    use crate::migrate::LEGACY;

    let unit = format!("/etc/systemd/system/{LEGACY}.service");
    if !Path::new(&unit).exists() {
        return;
    }
    let _ = systemctl(&["disable", "--now", &format!("{LEGACY}.service")]);
    let _ = std::fs::remove_file(&unit);
    let _ = systemctl(&["daemon-reload"]);
    let _ = std::fs::remove_dir_all(format!("/usr/local/lib/{LEGACY}"));
    crate::nfqueue::remove_table(LEGACY);
    println!("removed the old {LEGACY} service");
}

/// The unit `install` writes; packages ship the same text (`dpimech-service unit`).
pub fn unit_file(exe: &Path, data_dir: &Path) -> String {
    let mut exec = quote(exe);
    exec.push_str(" service");
    if data_dir != default_service_data_dir() {
        exec.push_str(" --data-dir ");
        exec.push_str(&quote(data_dir));
    }
    format!(
        "\
[Unit]
Description=DPIMech Service
Documentation=https://github.com/{repo}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={exec}
Restart=on-failure
RestartSec=5
# Engines are children of the service: stopping or crashing takes all of them down.
KillMode=control-group
RuntimeDirectory=dpimech
RuntimeDirectoryMode=0755
NoNewPrivileges=yes
ProtectSystem=full
ProtectHome=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
",
        repo = dpimech_core::catalog::APP_REPO,
    )
}

/// systemd splits ExecStart on whitespace and expands `%` and `$`.
fn quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "$$");
    format!("\"{escaped}\"")
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

/// Writes via a temporary file and rename, so a running old binary (busy text file) or a
/// half-written unit is never a problem, and ownership/mode are set before it is visible.
fn write_root_file(path: &Path, bytes: &[u8], mode: u32) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = path.parent().context("path has no parent")?;
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))?;
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

fn systemctl(args: &[&str]) -> anyhow::Result<()> {
    let out = Command::new("systemctl")
        .args(args)
        .output()
        .context("running systemctl (is this a systemd system?)")?;
    if !out.status.success() {
        bail!(
            "systemctl {} failed: {}",
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
    fn unit_quotes_paths_and_adds_custom_data_dir() {
        let unit = unit_file(
            Path::new("/usr/local/lib/dpimech/dpimech-service"),
            Path::new("/srv/dpi data/100%"),
        );
        assert!(unit.contains(
            "ExecStart=\"/usr/local/lib/dpimech/dpimech-service\" service --data-dir \"/srv/dpi data/100%%\"\n"
        ));
    }

    #[test]
    fn unit_omits_default_data_dir() {
        let unit = unit_file(Path::new("/x/dpimech-service"), &default_service_data_dir());
        assert!(unit.contains("ExecStart=\"/x/dpimech-service\" service\n"));
    }
}
