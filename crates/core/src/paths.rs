use std::path::PathBuf;

use crate::packages::PackageId;

/// Environment override for the service data directory (useful for development).
pub const DATA_DIR_ENV: &str = "DPIMECH_DATA_DIR";

/// Machine-wide directory owned by the service: config, engines, logs.
pub fn default_service_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(DATA_DIR_ENV) {
        return PathBuf::from(dir);
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into());
        PathBuf::from(base).join("dpimech")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/dpimech")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        PathBuf::from("/var/lib/dpimech")
    }
}

#[derive(Debug, Clone)]
pub struct DataDir {
    pub root: PathBuf,
}

impl DataDir {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Host lists and fake-packet files engines may read (`-H`, `-l`, …).
    pub fn lists_dir(&self) -> PathBuf {
        self.root.join("lists")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    /// Verified DPIMech updates waiting to be installed.
    pub fn updates_dir(&self) -> PathBuf {
        self.root.join("updates")
    }

    /// Versioned install root of an archive package: `<data>/engines/<slug>/`.
    /// Packages are only ever launched from here, never from a client-supplied path,
    /// because the service runs privileged.
    pub fn package_dir(&self, package: PackageId) -> PathBuf {
        self.root.join("engines").join(package.slug())
    }
}
