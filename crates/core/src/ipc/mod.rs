//! Line-delimited JSON protocol between the GUI (client) and the service (server).
//! Transport: named pipe on Windows, Unix socket elsewhere.

mod client;
mod transport;

pub use client::{Client, ClientError};
pub use transport::{Listener, connect};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, Lines};

use crate::lab::{IspInfo, LabRequest, LabResult, LabStrategy};
use crate::model::EngineKind;
use crate::model::{LogLine, Profile, ProfileState, ProfileStatus};
use crate::packages::{PackageId, PackageInfo};

/// Frames longer than this are rejected to protect the service from a misbehaving client.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Hello {
        client_version: String,
    },
    ListProfiles,
    SaveProfile {
        profile: Profile,
    },
    DeleteProfile {
        id: String,
    },
    StartProfile {
        id: String,
    },
    StopProfile {
        id: String,
    },
    RecentLogs,
    ListPackages,
    /// Refreshes latest-release info from GitHub.
    CheckUpdates,
    InstallPackage {
        id: PackageId,
    },
    RemovePackage {
        id: PackageId,
    },
    /// The sites offered for profiles and the Lab: fetched from the repository, else built in.
    DomainPacks,
    /// Built-in + cached online strategies for an engine.
    LabStrategies {
        engine: EngineKind,
    },
    /// Re-downloads the online strategy lists, then replies like `LabStrategies`.
    RefreshLabStrategies {
        engine: EngineKind,
    },
    /// Looks up the user's ISP (sends the public IP to a lookup service).
    DetectIsp,
    StartLab {
        request: LabRequest,
    },
    CancelLab,
    /// Checks GitHub for a newer DPIMech release.
    CheckAppUpdate,
    /// Downloads the newest release in `format` into the service's data folder and verifies
    /// it; answered with `AppUpdateReady` (or an error when nothing newer exists).
    PrepareAppUpdate {
        format: UpdateFormat,
    },
    /// Windows: starts the installer prepared by `PrepareAppUpdate`, silently. The service is
    /// stopped and restarted by the installer, so the connection drops.
    InstallAppUpdate,
    /// Adds DPIMech's engine folder to Windows Security exclusions (WinDivert is often
    /// flagged). Only on an explicit user request.
    AddDefenderExclusion,
    /// Whether the engine folder is already excluded (then the GUI hides its hint).
    DefenderStatus,
    /// Runs the connection check of a running profile right now.
    CheckProfile {
        id: String,
    },
    /// Turns the detailed (troubleshooting) log on or off until the service restarts.
    SetDetailedLog {
        on: bool,
    },
    /// Other DPI bypass tools running outside DPIMech (they clash with its engines).
    ForeignTools,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Reply {
    Hello {
        service_version: String,
        data_dir: String,
    },
    Profiles {
        profiles: Vec<ProfileState>,
    },
    DomainPacks {
        packs: Vec<crate::catalog::DomainPack>,
    },
    Logs {
        lines: Vec<LogLine>,
    },
    Packages {
        packages: Vec<PackageInfo>,
    },
    LabStrategies {
        strategies: Vec<LabStrategy>,
    },
    Isp {
        info: IspInfo,
    },
    AppUpdate {
        current: String,
        latest: Option<String>,
        url: String,
    },
    AppUpdateReady {
        version: String,
        /// The downloaded file (readable by the GUI; an AppImage is copied from here).
        path: String,
    },
    ForeignTools {
        tools: Vec<ForeignTool>,
    },
    DefenderStatus {
        excluded: bool,
    },
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    ProfilesChanged,
    PackagesChanged,
    AppUpdateAvailable {
        version: String,
        url: String,
    },
    LabProgress {
        done: u32,
        total: u32,
        result: Option<LabResult>,
    },
    LabFinished {
        cancelled: bool,
        error: Option<String>,
    },
    ProfileStatus {
        id: String,
        status: ProfileStatus,
    },
    Log {
        line: LogLine,
    },
}

/// What a self-update downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateFormat {
    /// `dpimech-setup-<version>.exe`, run silently by the service.
    WindowsInstaller,
    /// `DPIMech-<version>-x86_64.AppImage`, swapped in by the GUI (it owns the file).
    AppImage,
}

impl UpdateFormat {
    pub fn asset_name(self, version: &str) -> String {
        let version = version.trim_start_matches('v');
        match self {
            UpdateFormat::WindowsInstaller => format!("dpimech-setup-{version}.exe"),
            UpdateFormat::AppImage => format!("DPIMech-{version}-x86_64.AppImage"),
        }
    }
}

/// A DPI tool found running outside DPIMech's engine folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignTool {
    /// e.g. "GoodbyeDPI".
    pub name: String,
    /// Executable path, or the process name when the path is not readable.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientFrame {
    pub id: u64,
    pub request: Request,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerFrame {
    Reply {
        id: u64,
        result: Result<Reply, String>,
    },
    Event {
        event: Event,
    },
}

pub async fn write_frame<W, T>(w: &mut W, frame: &T) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let mut line = serde_json::to_string(frame).map_err(std::io::Error::other)?;
    line.push('\n');
    w.write_all(line.as_bytes()).await?;
    w.flush().await
}

/// Reads the next frame; `Ok(None)` means the peer closed the connection.
pub async fn read_frame<R, T>(lines: &mut Lines<BufReader<R>>) -> std::io::Result<Option<T>>
where
    R: tokio::io::AsyncRead + Unpin,
    T: for<'de> Deserialize<'de>,
{
    let Some(line) = lines.next_line().await? else {
        return Ok(None);
    };
    if line.len() > MAX_FRAME_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    serde_json::from_str(&line)
        .map(Some)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn frame_lines<R: tokio::io::AsyncRead>(r: R) -> Lines<BufReader<R>> {
    BufReader::new(r).lines()
}
