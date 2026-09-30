//! Windows Security (Defender) exclusion for DPIMech's engine folder.
//!
//! WinDivert, used by zapret and GoodbyeDPI, is a packet driver that antivirus products often
//! flag although it is a legitimate, signed driver. Excluding only `<data>\engines` (admin-only
//! writable, see acl.rs) keeps downloads from being quarantined. Never done automatically.

use std::path::Path;

use anyhow::bail;

#[cfg(windows)]
pub async fn exclude(dir: &Path) -> anyhow::Result<()> {
    use anyhow::Context;

    std::fs::create_dir_all(dir)?;
    // Single-quoted PowerShell literal; the path comes from the service's own data dir.
    let path = dir.display().to_string().replace('\'', "''");
    let mut cmd = tokio::process::Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &format!("Add-MpPreference -ExclusionPath '{path}'"),
    ]);
    cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    let out = cmd.output().await.context("running PowerShell")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!(
            "Windows Security refused the exclusion (is another antivirus in charge?): {}",
            err.trim()
        );
    }
    Ok(())
}

#[cfg(not(windows))]
pub async fn exclude(_dir: &Path) -> anyhow::Result<()> {
    bail!("only needed on Windows")
}

/// Asks Windows Security itself rather than remembering our own success: the user may have
/// removed the exclusion since, and then the hint should come back. `false` also when
/// another antivirus is in charge (Get-MpPreference fails), which keeps the hint visible.
#[cfg(windows)]
pub async fn is_excluded(dir: &Path) -> bool {
    let path = dir.display().to_string().replace('\'', "''");
    let mut cmd = tokio::process::Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        // -contains compares case-insensitively, like Windows paths.
        &format!(
            "$p = (Get-MpPreference).ExclusionPath; if ($p -and ($p -contains '{path}' -or $p -contains '{path}\\')) {{ 'yes' }} else {{ 'no' }}"
        ),
    ]);
    cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    cmd.output()
        .await
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim() == "yes")
}

#[cfg(not(windows))]
pub async fn is_excluded(_dir: &Path) -> bool {
    false
}
