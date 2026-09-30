//! Windows Firewall rule for ProxiFyre.
//!
//! ProxiFyre receives redirected connections on a local listener; Windows Firewall treats
//! them as inbound. An interactive user gets an "allow access?" prompt, but the service runs
//! in session 0 where no prompt can appear, so traffic was silently dropped. The service
//! therefore owns one rule for the exact ProxiFyre binary it installed (the official MSI
//! does the same). Needs admin rights; in unelevated development runs this fails harmlessly
//! and the usual prompt appears instead.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

const RULE_NAME: &str = "DPIMech ProxiFyre";

/// Path the rule currently points to, so the rule is rewritten only when it changes.
static CURRENT: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn allow_proxifyre(exe: &Path) {
    let mut current = CURRENT.lock().unwrap();
    if current.as_deref() == Some(exe) {
        return;
    }
    remove_rule();
    let program = format!("program={}", exe.display());
    let result = netsh(&[
        "advfirewall",
        "firewall",
        "add",
        "rule",
        &format!("name={RULE_NAME}"),
        "dir=in",
        "action=allow",
        &program,
        "enable=yes",
        "profile=any",
    ]);
    match result {
        Ok(()) => *current = Some(exe.to_path_buf()),
        Err(e) => tracing::warn!("could not add firewall rule for ProxiFyre: {e}"),
    }
}

pub fn remove_proxifyre_rule() {
    remove_rule();
    *CURRENT.lock().unwrap() = None;
}

/// The rule created by 0.1.x under the old name.
pub fn remove_legacy_rule() {
    let _ = netsh(&[
        "advfirewall",
        "firewall",
        "delete",
        "rule",
        "name=dpimngr ProxiFyre",
    ]);
}

fn remove_rule() {
    // "No rules match" is the normal case on first use.
    let _ = netsh(&[
        "advfirewall",
        "firewall",
        "delete",
        "rule",
        &format!("name={RULE_NAME}"),
    ]);
}

fn netsh(args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("netsh");
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    }
}
