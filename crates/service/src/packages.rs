//! Downloads, verifies and installs packages from GitHub Releases.
//!
//! Archive layout: `<data>/engines/<slug>/<version>/...` plus `installed.json` pointing at the
//! current version. The previous version is kept for rollback; older ones are deleted.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use dpimech_core::ipc::Event;
use dpimech_core::model::Os;
use dpimech_core::packages::{ExtractRule, PackageId, PackageInfo, PackageKind, PackageTask};
use dpimech_core::paths::DataDir;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, broadcast};

use crate::logs::LogBus;

const SOURCE: &str = "updater";
const USER_AGENT: &str = concat!("dpimech/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    /// `sha256:<hex>`, provided by GitHub for release assets.
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Installed {
    version: String,
    asset: String,
    sha256: String,
    /// Path of the main binary relative to the version directory.
    binary: Option<PathBuf>,
    previous: Option<String>,
}

#[derive(Clone)]
pub struct Packages {
    data: Arc<DataDir>,
    http: reqwest::Client,
    latest: Arc<Mutex<HashMap<PackageId, Release>>>,
    tasks: Arc<Mutex<HashMap<PackageId, PackageTask>>>,
    events: broadcast::Sender<Event>,
    logs: Arc<LogBus>,
}

impl Packages {
    pub fn new(data: Arc<DataDir>, events: broadcast::Sender<Event>, logs: Arc<LogBus>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(15))
            .build()
            .expect("http client");
        Self {
            data,
            http,
            latest: Arc::default(),
            tasks: Arc::default(),
            events,
            logs,
        }
    }

    pub async fn list(&self) -> Vec<PackageInfo> {
        let latest = self.latest.lock().await;
        let tasks = self.tasks.lock().await;
        let mut out = Vec::new();
        for id in PackageId::available_on(Os::current()) {
            let release = latest.get(&id);
            out.push(PackageInfo {
                id,
                installed_version: self.installed_version(id),
                latest_version: release.map(|r| r.tag_name.clone()),
                latest_published: release.and_then(|r| r.published_at.clone()),
                latest_notes: release.and_then(|r| r.body.clone()),
                task: tasks.get(&id).cloned().unwrap_or(PackageTask::Idle),
            });
        }
        out
    }

    pub async fn check_updates(&self) -> anyhow::Result<()> {
        let mut errors = Vec::new();
        for id in PackageId::available_on(Os::current()) {
            match self.fetch_latest(id).await {
                Ok(release) => {
                    self.latest.lock().await.insert(id, release);
                }
                Err(e) => errors.push(format!("{}: {e:#}", id.display_name())),
            }
        }
        let _ = self.events.send(Event::PackagesChanged);
        if errors.is_empty() {
            self.logs.info(SOURCE, "update check finished");
            Ok(())
        } else {
            bail!(errors.join("; "))
        }
    }

    /// Starts an install in the background; progress is reported via `PackagesChanged`.
    pub async fn install(&self, id: PackageId) -> anyhow::Result<()> {
        if !id.supported_os().contains(&Os::current()) {
            bail!("{} is not available on this OS", id.display_name());
        }
        {
            let mut tasks = self.tasks.lock().await;
            if matches!(
                tasks.get(&id),
                Some(PackageTask::Downloading { .. } | PackageTask::Installing)
            ) {
                bail!("{} is already being installed", id.display_name());
            }
            tasks.insert(id, PackageTask::Downloading { percent: 0 });
        }
        let _ = self.events.send(Event::PackagesChanged);

        let this = self.clone();
        tokio::spawn(async move {
            let result = this.install_inner(id).await;
            let task = match result {
                Ok(version) => {
                    this.logs
                        .info(SOURCE, format!("{} {version} installed", id.display_name()));
                    PackageTask::Idle
                }
                Err(e) => {
                    let message = format!("{e:#}");
                    this.logs.error(
                        SOURCE,
                        format!("{} install failed: {message}", id.display_name()),
                    );
                    PackageTask::Failed { message }
                }
            };
            this.tasks.lock().await.insert(id, task);
            let _ = this.events.send(Event::PackagesChanged);
        });
        Ok(())
    }

    pub async fn remove(&self, id: PackageId) -> anyhow::Result<()> {
        match id.kind() {
            PackageKind::Archive => {
                let dir = self.data.package_dir(id);
                if dir.exists() {
                    std::fs::remove_dir_all(&dir)
                        .with_context(|| format!("removing {}", dir.display()))?;
                }
                #[cfg(windows)]
                if id == PackageId::ProxiFyre {
                    crate::firewall::remove_proxifyre_rule();
                }
            }
            PackageKind::DriverMsi => {
                bail!("uninstall the driver from Windows Settings → Apps")
            }
        }
        self.tasks.lock().await.remove(&id);
        let _ = self.events.send(Event::PackagesChanged);
        self.logs
            .info(SOURCE, format!("{} removed", id.display_name()));
        Ok(())
    }

    /// Absolute path of the installed main binary, if any.
    pub fn binary_path(&self, id: PackageId) -> Option<PathBuf> {
        let installed = self.read_installed(id)?;
        let path = self
            .data
            .package_dir(id)
            .join(&installed.version)
            .join(installed.binary?);
        path.is_file().then_some(path)
    }

    /// Executable of one engine: the package's main binary or a sibling of it.
    pub fn engine_path(&self, engine: dpimech_core::model::EngineKind) -> Option<PathBuf> {
        let main = self.binary_path(PackageId::for_engine(engine)?)?;
        let name = if cfg!(windows) {
            format!("{}.exe", engine.binary_stem())
        } else {
            engine.binary_stem().to_owned()
        };
        let path = main.with_file_name(name);
        if path.is_file() {
            return Some(path);
        }
        // Unix builds may carry the CPU in the name (`ciadpi-x86_64`); that is the main binary.
        main.file_stem()
            .is_some_and(|s| s.to_string_lossy().starts_with(engine.binary_stem()))
            .then_some(main)
    }

    pub fn installed_version(&self, id: PackageId) -> Option<String> {
        match id.kind() {
            PackageKind::Archive => self.read_installed(id).map(|i| i.version),
            PackageKind::DriverMsi => driver_version(),
        }
    }

    fn read_installed(&self, id: PackageId) -> Option<Installed> {
        let text =
            std::fs::read_to_string(self.data.package_dir(id).join("installed.json")).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Latest DPIMech release tag, if a newer one than this build exists.
    pub async fn check_app_update(&self) -> anyhow::Result<Option<String>> {
        let release = match self
            .fetch_repo_latest(dpimech_core::catalog::APP_REPO)
            .await
        {
            Ok(r) => r,
            // No public release yet (or the repository is private).
            Err(e) if e.to_string().contains("404") => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(
            dpimech_core::packages::version_lt(dpimech_core::VERSION, &release.tag_name)
                .then_some(release.tag_name),
        )
    }

    async fn fetch_latest(&self, id: PackageId) -> anyhow::Result<Release> {
        self.fetch_repo_latest(id.repo()).await
    }

    async fn fetch_repo_latest(&self, repo: &str) -> anyhow::Result<Release> {
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        let response = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            bail!("GitHub rate limit reached, try again later");
        }
        Ok(response.error_for_status()?.json().await?)
    }

    async fn set_task(&self, id: PackageId, task: PackageTask) {
        self.tasks.lock().await.insert(id, task);
        let _ = self.events.send(Event::PackagesChanged);
    }

    async fn install_inner(&self, id: PackageId) -> anyhow::Result<String> {
        let release = self.fetch_latest(id).await?;
        self.latest.lock().await.insert(id, release.clone());
        let asset = release
            .assets
            .iter()
            .find(|a| id.matches_asset(&a.name, Os::current(), std::env::consts::ARCH))
            .with_context(|| format!("no download for this system in {}", release.tag_name))?;
        let expected = asset
            .digest
            .as_deref()
            .and_then(|d| d.strip_prefix("sha256:"))
            .or_else(|| id.pinned_sha256(&asset.name))
            .context("release asset has no SHA-256 digest; refusing to install")?
            .to_ascii_lowercase();

        let downloads = self.data.downloads_dir();
        tokio::fs::create_dir_all(&downloads).await?;
        let file = downloads.join(&asset.name);
        let actual = self.download(id, asset, &file).await?;
        if actual != expected {
            let _ = tokio::fs::remove_file(&file).await;
            bail!("checksum mismatch (expected {expected}, got {actual})");
        }

        self.set_task(id, PackageTask::Installing).await;
        let version = release.tag_name.clone();
        let result = match id.kind() {
            PackageKind::Archive => {
                let data = self.data.clone();
                let (file, asset_name, version) =
                    (file.clone(), asset.name.clone(), version.clone());
                tokio::task::spawn_blocking(move || {
                    install_archive(&data, id, &file, &asset_name, &version, &actual)
                })
                .await?
            }
            PackageKind::DriverMsi => install_msi(&file).await,
        };
        let _ = tokio::fs::remove_file(&file).await;
        result.map(|()| version)
    }

    /// Streams the asset to disk and returns its SHA-256.
    async fn download(&self, id: PackageId, asset: &Asset, dest: &Path) -> anyhow::Result<String> {
        let response = self
            .http
            .get(&asset.browser_download_url)
            .timeout(Duration::from_secs(600))
            .send()
            .await?
            .error_for_status()?;
        let total = response.content_length().unwrap_or(asset.size).max(1);
        let mut out = tokio::fs::File::create(dest).await?;
        let mut hasher = Sha256::new();
        let mut received = 0u64;
        let mut last_percent = 0u8;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            hasher.update(&chunk);
            out.write_all(&chunk).await?;
            received += chunk.len() as u64;
            let percent = ((received * 100) / total).min(100) as u8;
            if percent >= last_percent + 5 {
                last_percent = percent;
                self.set_task(id, PackageTask::Downloading { percent })
                    .await;
            }
        }
        out.flush().await?;
        Ok(hex::encode(hasher.finalize()))
    }
}

fn install_archive(
    data: &DataDir,
    id: PackageId,
    file: &Path,
    asset_name: &str,
    version: &str,
    sha256: &str,
) -> anyhow::Result<()> {
    let root = data.package_dir(id);
    let target = root.join(version);
    let staging = root.join(format!(".{version}.staging"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;

    let rules = id.extract_rules(Os::current(), std::env::consts::ARCH);
    if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        extract_tar_gz(file, &staging, rules)?;
    } else {
        extract_zip(file, &staging, rules)?;
    }

    let binary = id
        .binary_stem(Os::current())
        .map(|stem| find_binary(&staging, stem))
        .transpose()?;
    #[cfg(unix)]
    if let Some(main) = &binary {
        mark_engines_executable(id, &staging.join(main))?;
    }

    if target.exists() && std::fs::remove_dir_all(&target).is_err() {
        // Reinstalling the version a running engine is using: its files are locked, but they
        // are the same version, so only restore whatever is missing.
        fill_missing(&staging, &target)?;
        let _ = std::fs::remove_dir_all(&staging);
    } else {
        std::fs::rename(&staging, &target).with_context(|| {
            format!(
                "{} files are in use — stop profiles using it and retry",
                id.display_name()
            )
        })?;
    }

    let previous = std::fs::read_to_string(root.join("installed.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Installed>(&t).ok())
        .map(|i| i.version)
        .filter(|v| v != version);
    let installed = Installed {
        version: version.to_owned(),
        asset: asset_name.to_owned(),
        sha256: sha256.to_owned(),
        binary,
        previous: previous.clone(),
    };
    let tmp = root.join("installed.json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&installed)?)?;
    std::fs::rename(&tmp, root.join("installed.json"))?;

    // Keep current + previous; files of a running older version may be locked, so ignore errors.
    for entry in std::fs::read_dir(&root)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let keep = name == version || Some(&name) == previous.as_ref();
        if entry.path().is_dir() && !keep && !name.starts_with('.') {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
}

fn extract_zip(file: &Path, staging: &Path, rules: Option<&[ExtractRule]>) -> anyhow::Result<()> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(file)?)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        // enclosed_name rejects absolute paths and `..` (zip-slip).
        let Some(rel) = entry.enclosed_name() else {
            bail!("archive contains an unsafe path: {}", entry.name());
        };
        let Some(out_path) = destination(staging, rel, rules) else {
            continue;
        };
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            let mode = entry.unix_mode();
            write_file(&out_path, &mut entry, mode)?;
        }
    }
    Ok(())
}

/// Only plain files and directories are taken: a symlink or hard link could point the
/// service at a file outside the package.
fn extract_tar_gz(
    file: &Path,
    staging: &Path,
    rules: Option<&[ExtractRule]>,
) -> anyhow::Result<()> {
    let gz = flate2::read::GzDecoder::new(std::fs::File::open(file)?);
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let rel = entry.path()?.into_owned();
        if !rel
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        {
            bail!("archive contains an unsafe path: {}", rel.display());
        }
        let rel: PathBuf = rel
            .components()
            .filter(|c| *c != Component::CurDir)
            .collect();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let kind = entry.header().entry_type();
        let Some(out_path) = destination(staging, rel, rules) else {
            continue;
        };
        if kind.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else if kind.is_file() {
            let mode = entry.header().mode().ok();
            write_file(&out_path, &mut entry, mode)?;
        }
    }
    Ok(())
}

fn destination(staging: &Path, rel: PathBuf, rules: Option<&[ExtractRule]>) -> Option<PathBuf> {
    match rules {
        None => Some(staging.join(rel)),
        Some(rules) => map_entry(&rel, rules).map(|mapped| staging.join(mapped)),
    }
}

fn write_file(path: &Path, from: &mut impl std::io::Read, mode: Option<u32>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = std::fs::File::create(path)?;
    std::io::copy(from, &mut out)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Keep the executable bits, drop setuid/setgid/sticky and group/other write.
        let mode = mode.map_or(0o644, |m| m & 0o755);
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}

/// Zip entries made on Windows carry no Unix mode, which would leave the engines
/// non-executable; every engine of the package (zapret: nfqws and tpws) gets 0755.
#[cfg(unix)]
fn mark_engines_executable(id: PackageId, main: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = main.parent().context("binary has no parent")?;
    let engines = dpimech_core::model::EngineKind::ALL
        .into_iter()
        .filter(|e| PackageId::for_engine(*e) == Some(id))
        .map(|e| dir.join(e.binary_stem()));
    for path in std::iter::once(main.to_path_buf()).chain(engines) {
        if path.is_file() {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

/// Applies extract rules to an archive path: strips the archive's top-level folder
/// (`zapret-v72.13/`), keeps only entries under a rule's `from`, and re-roots them at `to`.
fn map_entry(rel: &Path, rules: &[ExtractRule]) -> Option<PathBuf> {
    let inner: PathBuf = rel.components().skip(1).collect();
    let inner = inner.to_string_lossy().replace('\\', "/");
    rules.iter().find_map(|rule| {
        inner
            .strip_prefix(rule.from)
            .filter(|rest| !rest.is_empty())
            .map(|rest| PathBuf::from(format!("{}{rest}", rule.to)))
    })
}

/// Copies files that exist in `from` but not in `to`, leaving existing (possibly locked) ones.
fn fill_missing(from: &Path, to: &Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(from)?.flatten() {
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            std::fs::create_dir_all(&dest)?;
            fill_missing(&entry.path(), &dest)?;
        } else if !dest.exists() {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// Finds `<stem>[.exe]` anywhere under `dir` and returns it relative to `dir`. Unix builds
/// are also accepted as `<stem>-<arch>`, the name CI release scripts commonly give them.
fn find_binary(dir: &Path, stem: &str) -> anyhow::Result<PathBuf> {
    let wanted: Vec<String> = if cfg!(windows) {
        vec![format!("{stem}.exe")]
    } else {
        vec![
            stem.to_owned(),
            format!("{stem}-{}", std::env::consts::ARCH),
        ]
    };
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)?.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                stack.push(path);
            } else if wanted.iter().any(|w| name.eq_ignore_ascii_case(w)) {
                return Ok(path.strip_prefix(dir)?.to_path_buf());
            }
        }
    }
    bail!("{} not found in the downloaded archive", wanted[0])
}

#[cfg(windows)]
async fn install_msi(file: &Path) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("msiexec")
        .arg("/i")
        .arg(file)
        .args(["/qn", "/norestart"])
        .status()
        .await?;
    // 3010 = success, reboot required.
    match status.code() {
        Some(0) | Some(3010) => Ok(()),
        Some(1925) | Some(1603) => {
            bail!("installer failed (needs administrator rights?) — code {status}")
        }
        _ => bail!("installer exited with {status}"),
    }
}

#[cfg(not(windows))]
async fn install_msi(_file: &Path) -> anyhow::Result<()> {
    bail!("MSI packages are Windows-only")
}

/// Version of the installed Windows Packet Filter, read from the uninstall registry.
#[cfg(windows)]
pub fn driver_version() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for path in [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ] {
        let Ok(root) = hklm.open_subkey(path) else {
            continue;
        };
        for name in root.enum_keys().flatten() {
            let Ok(key) = root.open_subkey(&name) else {
                continue;
            };
            let display: String = key.get_value("DisplayName").unwrap_or_default();
            if display.starts_with("Windows Packet Filter") {
                return key.get_value("DisplayVersion").ok();
            }
        }
    }
    None
}

#[cfg(not(windows))]
pub fn driver_version() -> Option<String> {
    None
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn tarball(dir: &Path, build: impl FnOnce(&mut tar::Builder<Vec<u8>>)) -> PathBuf {
        let mut builder = tar::Builder::new(Vec::new());
        build(&mut builder);
        let raw = builder.into_inner().unwrap();
        let path = dir.join("pkg.tar.gz");
        let mut gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&path).unwrap(),
            flate2::Compression::fast(),
        );
        std::io::Write::write_all(&mut gz, &raw).unwrap();
        gz.finish().unwrap();
        path
    }

    fn file(builder: &mut tar::Builder<Vec<u8>>, path: &str, mode: u32) {
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(mode);
        header.set_entry_type(tar::EntryType::Regular);
        builder
            .append_data(&mut header, path, &b"test"[..])
            .unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpimech-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn tar_keeps_exec_bit_drops_setuid_and_links() {
        let dir = temp_dir("tar-ok");
        let archive = tarball(&dir, |b| {
            file(b, "byedpi-17/ciadpi-x86_64", 0o4777);
            file(b, "byedpi-17/README.md", 0o666);
            let mut link = tar::Header::new_gnu();
            link.set_entry_type(tar::EntryType::Symlink);
            link.set_size(0);
            b.append_link(&mut link, "byedpi-17/evil", "/etc/shadow")
                .unwrap();
        });
        let out = dir.join("out");
        extract_tar_gz(&archive, &out, None).unwrap();

        let bin = out.join("byedpi-17/ciadpi-x86_64");
        assert_eq!(bin.metadata().unwrap().permissions().mode() & 0o7777, 0o755);
        let readme = out.join("byedpi-17/README.md");
        assert_eq!(
            readme.metadata().unwrap().permissions().mode() & 0o7777,
            0o644
        );
        assert!(std::fs::symlink_metadata(out.join("byedpi-17/evil")).is_err());

        // `ciadpi-<arch>` is found as the ciadpi binary on this CPU.
        if std::env::consts::ARCH == "x86_64" {
            assert_eq!(
                find_binary(&out, "ciadpi").unwrap(),
                PathBuf::from("byedpi-17/ciadpi-x86_64")
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tar_rejects_parent_paths() {
        let dir = temp_dir("tar-slip");
        let archive = tarball(&dir, |b| {
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_mode(0o644);
            header.set_entry_type(tar::EntryType::Regular);
            // append_data refuses `..`, so the name is written into the header directly.
            header.as_old_mut().name[..9].copy_from_slice(b"../escape");
            header.set_cksum();
            b.append(&header, &b"test"[..]).unwrap();
        });
        let out = dir.join("out");
        assert!(extract_tar_gz(&archive, &out, None).is_err());
        assert!(!dir.join("escape").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zapret_engines_become_executable() {
        let dir = temp_dir("exec");
        for name in ["nfqws", "tpws", "ip2net"] {
            std::fs::write(dir.join(name), "x").unwrap();
            std::fs::set_permissions(dir.join(name), std::fs::Permissions::from_mode(0o644))
                .unwrap();
        }
        mark_engines_executable(PackageId::Zapret, &dir.join("nfqws")).unwrap();
        let mode = |n: &str| dir.join(n).metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode("nfqws"), 0o755);
        assert_eq!(mode("tpws"), 0o755);
        assert_eq!(
            mode("ip2net"),
            0o644,
            "helpers DPIMech never runs stay as they are"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tar_applies_extract_rules() {
        let dir = temp_dir("tar-rules");
        let archive = tarball(&dir, |b| {
            file(b, "zapret-v72/binaries/linux-x86_64/nfqws", 0o755);
            file(b, "zapret-v72/binaries/windows-x86_64/winws.exe", 0o755);
            file(b, "zapret-v72/files/fake/quic.bin", 0o644);
        });
        let rules = [
            ExtractRule {
                from: "binaries/linux-x86_64/",
                to: "",
            },
            ExtractRule {
                from: "files/fake/",
                to: "fake/",
            },
        ];
        let out = dir.join("out");
        extract_tar_gz(&archive, &out, Some(&rules)).unwrap();
        assert!(out.join("nfqws").is_file());
        assert!(out.join("fake/quic.bin").is_file());
        assert!(!out.join("winws.exe").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
