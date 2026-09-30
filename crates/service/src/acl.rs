//! Locks down the service data directory.
//!
//! `C:\ProgramData` lets standard users create files in new subfolders. Without this, a user
//! could plant `engines\...\installed.json` pointing at their own executable and have the
//! service start it as SYSTEM. The directory is reset to: SYSTEM and Administrators full
//! control, Users read-only, no inheritance from ProgramData, and every child's explicit ACEs
//! and ownership replaced. SIDs are used instead of names, which are localised
//! ("Users" is "Kullanıcılar" on Turkish Windows).
//!
//! On Linux/macOS the same attack works if the directory was pre-created by a user or made
//! writable later: everything is reset to root-owned, not writable by group or others.

#[cfg(windows)]
pub use windows::harden_data_dir;

#[cfg(unix)]
pub use unix::harden_data_dir;

#[cfg(windows)]
mod windows {
    use std::path::Path;
    use std::process::Command;

    use anyhow::{Context, bail};

    const SYSTEM: &str = "*S-1-5-18";
    const ADMINISTRATORS: &str = "*S-1-5-32-544";
    const USERS: &str = "*S-1-5-32-545";
    /// OWNER RIGHTS: strips the implicit WRITE_DAC a file's owner would otherwise keep.
    const OWNER_RIGHTS: &str = "*S-1-3-4";

    pub fn harden_data_dir(dir: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(dir)?;
        let dir = dir.to_string_lossy();

        // Administrators own everything, including anything a user may have pre-created.
        run("takeown", &["/F", &dir, "/R", "/A", "/D", "Y"])?;
        run(
            "icacls",
            &[
                &dir,
                "/inheritance:r",
                "/grant:r",
                &format!("{SYSTEM}:(OI)(CI)F"),
                &format!("{ADMINISTRATORS}:(OI)(CI)F"),
                &format!("{USERS}:(OI)(CI)RX"),
                &format!("{OWNER_RIGHTS}:(OI)(CI)(RC)"),
                "/Q",
            ],
        )?;
        // Children: drop explicit ACEs so they only inherit the rules above.
        run(
            "icacls",
            &[&format!("{dir}\\*"), "/reset", "/T", "/C", "/Q"],
        )?;
        Ok(())
    }

    fn run(program: &str, args: &[&str]) -> anyhow::Result<()> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
        }
        let out = cmd.output().with_context(|| format!("running {program}"))?;
        // `icacls dir\* /reset` on an empty directory reports "no files" but is harmless.
        let stderr = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() && !stderr.trim().is_empty() {
            bail!("{program} failed: {}", stderr.trim());
        }
        Ok(())
    }
}

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::Path;

    use anyhow::{Context, bail};

    /// Only root can change ownership; a development service run by a user is left alone.
    pub fn harden_data_dir(dir: &Path) -> anyhow::Result<()> {
        // SAFETY: geteuid has no preconditions.
        if unsafe { libc::geteuid() } != 0 {
            return Ok(());
        }
        std::fs::create_dir_all(dir)?;
        if std::fs::symlink_metadata(dir)?.file_type().is_symlink() {
            bail!("{} is a symbolic link; refusing to use it", dir.display());
        }
        harden(dir)
    }

    fn harden(path: &Path) -> anyhow::Result<()> {
        let meta = std::fs::symlink_metadata(path)?;
        // The service never creates links; one found here was planted to redirect a
        // privileged write or launch, so it is removed rather than followed.
        if meta.file_type().is_symlink() {
            std::fs::remove_file(path)
                .with_context(|| format!("removing link {}", path.display()))?;
            return Ok(());
        }
        if meta.uid() != 0 || meta.gid() != 0 {
            std::os::unix::fs::lchown(path, Some(0), Some(0))
                .with_context(|| format!("chown {}", path.display()))?;
        }
        // Keep the execute bits engines need; drop setuid/setgid/sticky and group/other write.
        let mode = meta.mode() & 0o7777;
        if mode != mode & 0o755 {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode & 0o755))?;
        }
        if meta.is_dir() {
            for entry in std::fs::read_dir(path)? {
                harden(&entry?.path())?;
            }
        }
        Ok(())
    }
}
