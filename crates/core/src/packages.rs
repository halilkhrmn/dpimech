//! Catalog of downloadable components: DPI engines and their helpers.

use serde::{Deserialize, Serialize};

use crate::model::{EngineKind, Os};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageId {
    ByeDpi,
    Zapret,
    GoodbyeDpi,
    ProxiFyre,
    PacketFilterDriver,
    SpoofDpi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKind {
    /// Zip archive extracted into `<data>/engines/<slug>/<version>/`.
    Archive,
    /// Windows installer for a kernel driver; installed system-wide via msiexec.
    DriverMsi,
}

/// Which parts of an archive to keep: entries under `from` (after the archive's top-level
/// folder is stripped) are extracted to `to` inside the version directory.
#[derive(Debug, Clone, Copy)]
pub struct ExtractRule {
    pub from: &'static str,
    pub to: &'static str,
}

impl PackageId {
    pub const ALL: [PackageId; 6] = [
        PackageId::ByeDpi,
        PackageId::Zapret,
        PackageId::SpoofDpi,
        PackageId::GoodbyeDpi,
        PackageId::ProxiFyre,
        PackageId::PacketFilterDriver,
    ];

    pub const fn display_name(self) -> &'static str {
        match self {
            PackageId::ByeDpi => "ByeDPI",
            PackageId::Zapret => "zapret",
            PackageId::GoodbyeDpi => "GoodbyeDPI",
            PackageId::ProxiFyre => "ProxiFyre",
            PackageId::PacketFilterDriver => "Windows Packet Filter",
            PackageId::SpoofDpi => "SpoofDPI",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            PackageId::ByeDpi => {
                "SOCKS5 proxy that splits and disguises TLS/HTTP to slip past DPI."
            }
            PackageId::Zapret => {
                "Packet-level bypass (winws on Windows, nfqws + tpws on Linux). Handles TCP and UDP, incl. Discord voice and QUIC."
            }
            PackageId::GoodbyeDpi => {
                "Classic system-wide bypass for Windows (WinDivert). Simple, no longer actively developed."
            }
            PackageId::ProxiFyre => {
                "Routes selected apps through a local SOCKS5 proxy (per-app routing)."
            }
            PackageId::PacketFilterDriver => {
                "Kernel driver required by ProxiFyre. Installed system-wide."
            }
            PackageId::SpoofDpi => {
                "SOCKS5 proxy that splits TLS and can resolve names over HTTPS (DoH), which also gets around DNS blocking."
            }
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            PackageId::ByeDpi => "byedpi",
            PackageId::Zapret => "zapret",
            PackageId::GoodbyeDpi => "goodbyedpi",
            PackageId::ProxiFyre => "proxifyre",
            PackageId::PacketFilterDriver => "windows-packet-filter",
            PackageId::SpoofDpi => "spoofdpi",
        }
    }

    /// GitHub `owner/repo` that publishes releases.
    pub const fn repo(self) -> &'static str {
        match self {
            PackageId::ByeDpi => "hufrea/byedpi",
            PackageId::Zapret => "bol-van/zapret",
            PackageId::GoodbyeDpi => "ValdikSS/GoodbyeDPI",
            PackageId::ProxiFyre => "wiresock/proxifyre",
            PackageId::PacketFilterDriver => "wiresock/ndisapi",
            PackageId::SpoofDpi => "xvzc/SpoofDPI",
        }
    }

    pub const fn kind(self) -> PackageKind {
        match self {
            PackageId::PacketFilterDriver => PackageKind::DriverMsi,
            _ => PackageKind::Archive,
        }
    }

    pub const fn supported_os(self) -> &'static [Os] {
        match self {
            PackageId::ByeDpi => &[Os::Windows, Os::Linux, Os::MacOs],
            // macOS: only tpws exists for it in the archive (no NFQUEUE/WinDivert there).
            PackageId::Zapret => &[Os::Windows, Os::Linux, Os::MacOs],
            PackageId::GoodbyeDpi | PackageId::ProxiFyre | PackageId::PacketFilterDriver => {
                &[Os::Windows]
            }
            // SpoofDPI 1.x publishes Linux and macOS builds only.
            PackageId::SpoofDpi => &[Os::Linux, Os::MacOs],
        }
    }

    /// Main executable (without extension) inside an archive package. Other engines of the
    /// same package sit next to it (zapret: `tpws` beside `nfqws`).
    pub const fn binary_stem(self, os: Os) -> Option<&'static str> {
        match self {
            PackageId::ByeDpi => Some("ciadpi"),
            PackageId::Zapret => match os {
                Os::Windows => Some("winws"),
                Os::MacOs => Some("tpws"),
                Os::Linux => Some("nfqws"),
            },
            PackageId::GoodbyeDpi => Some("goodbyedpi"),
            PackageId::ProxiFyre => Some("ProxiFyre"),
            PackageId::PacketFilterDriver => None,
            PackageId::SpoofDpi => Some("spoofdpi"),
        }
    }

    /// `None` extracts the whole archive. zapret ships every platform in one zip, so only the
    /// binaries for this OS/CPU and the fake payloads are kept.
    pub fn extract_rules(self, os: Os, arch: &str) -> Option<&'static [ExtractRule]> {
        const fn zapret(binaries: &'static str) -> [ExtractRule; 2] {
            [
                ExtractRule {
                    from: binaries,
                    to: "",
                },
                ExtractRule {
                    from: "files/fake/",
                    to: "fake/",
                },
            ]
        }
        const WIN_X86: [ExtractRule; 2] = zapret("binaries/windows-x86/");
        const WIN_X64: [ExtractRule; 2] = zapret("binaries/windows-x86_64/");
        const LINUX_X64: [ExtractRule; 2] = zapret("binaries/linux-x86_64/");
        const LINUX_X86: [ExtractRule; 2] = zapret("binaries/linux-x86/");
        const LINUX_ARM64: [ExtractRule; 2] = zapret("binaries/linux-arm64/");
        const LINUX_ARM: [ExtractRule; 2] = zapret("binaries/linux-arm/");
        // One universal (x86_64 + arm64) build for macOS.
        const MAC: [ExtractRule; 2] = zapret("binaries/mac64/");
        match self {
            PackageId::Zapret => Some(match (os, arch) {
                (Os::Windows, "x86") => &WIN_X86,
                (Os::Windows, _) => &WIN_X64,
                (Os::MacOs, _) => &MAC,
                (_, "x86") => &LINUX_X86,
                (_, "aarch64") => &LINUX_ARM64,
                (_, "arm") => &LINUX_ARM,
                _ => &LINUX_X64,
            }),
            PackageId::GoodbyeDpi => Some(match arch {
                "x86" => &[ExtractRule {
                    from: "x86/",
                    to: "",
                }],
                _ => &[ExtractRule {
                    from: "x86_64/",
                    to: "",
                }],
            }),
            _ => None,
        }
    }

    /// SHA-256 for release assets published before GitHub started attaching digests.
    /// Anything else without a digest is refused.
    pub fn pinned_sha256(self, asset: &str) -> Option<&'static str> {
        match (self, asset) {
            (PackageId::GoodbyeDpi, "goodbyedpi-0.2.2.zip") => {
                Some("00a2f8b99cd817f8c7fc4c449033015f039d18af213de78cb66bf202277c0628")
            }
            _ => None,
        }
    }

    pub fn from_slug(slug: &str) -> Option<PackageId> {
        Self::ALL.into_iter().find(|p| p.slug() == slug)
    }

    pub fn for_engine(engine: EngineKind) -> Option<PackageId> {
        match engine {
            EngineKind::ByeDpi => Some(PackageId::ByeDpi),
            EngineKind::ZapretWinws => Some(PackageId::Zapret),
            EngineKind::GoodbyeDpi => Some(PackageId::GoodbyeDpi),
            EngineKind::ZapretNfqws | EngineKind::ZapretTpws => Some(PackageId::Zapret),
            EngineKind::SpoofDpi => Some(PackageId::SpoofDpi),
        }
    }

    pub fn available_on(os: Os) -> impl Iterator<Item = PackageId> {
        Self::ALL
            .into_iter()
            .filter(move |p| p.supported_os().contains(&os))
    }

    /// Picks the release asset for this OS/CPU. `arch` is `std::env::consts::ARCH`.
    pub fn matches_asset(self, name: &str, os: Os, arch: &str) -> bool {
        let n = name.to_ascii_lowercase();
        match (self, os) {
            (PackageId::ByeDpi, Os::Windows) => match arch {
                "x86_64" => n.ends_with("-x86_64-w64.zip"),
                _ => n.ends_with("-i686-w64.zip"),
            },
            (PackageId::ByeDpi, _) => n.ends_with(&format!("-{arch}.tar.gz")),
            // One archive for all platforms; "-openwrt-embedded" and ".tar.gz" are skipped.
            (PackageId::Zapret, _) => {
                n.starts_with("zapret-v") && n.ends_with(".zip") && !n.contains("openwrt")
            }
            (PackageId::GoodbyeDpi, Os::Windows) => {
                n.starts_with("goodbyedpi-") && n.ends_with(".zip")
            }
            (PackageId::ProxiFyre, Os::Windows) => {
                let suffix = match arch {
                    "x86_64" => "-x64.zip",
                    "aarch64" => "-arm64.zip",
                    _ => "-x86.zip",
                };
                n.starts_with("proxifyre-v") && n.ends_with(suffix)
            }
            // goreleaser: spoofdpi_<version>_<linux|darwin>_<x86_64|arm64|i386|arm>.tar.gz
            (PackageId::SpoofDpi, Os::Linux | Os::MacOs) => {
                let system = if os == Os::MacOs { "darwin" } else { "linux" };
                let cpu = match arch {
                    "aarch64" => "arm64",
                    "x86" => "i386",
                    other => other,
                };
                n.starts_with("spoofdpi_") && n.ends_with(&format!("_{system}_{cpu}.tar.gz"))
            }
            (PackageId::PacketFilterDriver, Os::Windows) => {
                let suffix = match arch {
                    "x86_64" => ".x64.msi",
                    "aarch64" => ".arm64.msi",
                    _ => ".x86.msi",
                };
                n.starts_with("windows.packet.filter.") && n.ends_with(suffix)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PackageTask {
    Idle,
    Downloading { percent: u8 },
    Installing,
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub id: PackageId,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub latest_published: Option<String>,
    pub latest_notes: Option<String>,
    pub task: PackageTask,
}

impl PackageInfo {
    pub fn update_available(&self) -> bool {
        match (&self.installed_version, &self.latest_version) {
            (Some(installed), Some(latest)) => version_lt(installed, latest),
            _ => false,
        }
    }
}

/// Compares dotted versions numerically, ignoring a leading `v` and trailing zero parts,
/// so `3.6.1.1` < `v3.6.2` and `v0.17.3` == `0.17.3.0`.
pub fn version_lt(a: &str, b: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        let mut p: Vec<u64> = v
            .trim_start_matches(['v', 'V'])
            .split(['.', '-'])
            .map_while(|s| s.parse().ok())
            .collect();
        while p.last() == Some(&0) {
            p.pop();
        }
        p
    }
    parts(a) < parts(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(version_lt("3.6.1.1", "v3.6.2"));
        assert!(!version_lt("v0.17.3", "0.17.3.0"));
        assert!(version_lt("v0.9", "v0.17.3"));
        assert!(!version_lt("v2.6.1", "v2.6.0"));
    }

    #[test]
    fn matches_windows_assets() {
        let w = Os::Windows;
        assert!(PackageId::ByeDpi.matches_asset("byedpi-17.3-x86_64-w64.zip", w, "x86_64"));
        assert!(!PackageId::ByeDpi.matches_asset("byedpi-17.3-x86_64.tar.gz", w, "x86_64"));
        assert!(PackageId::ProxiFyre.matches_asset("ProxiFyre-v2.6.1-x64.zip", w, "x86_64"));
        assert!(!PackageId::ProxiFyre.matches_asset(
            "ProxiFyre-v2.6.1-x64.zip.sha256",
            w,
            "x86_64"
        ));
        assert!(!PackageId::ProxiFyre.matches_asset("ProxiFyre-2.6.1-win-x64.msi", w, "x86_64"));
        assert!(PackageId::PacketFilterDriver.matches_asset(
            "Windows.Packet.Filter.3.6.2.1.x64.msi",
            w,
            "x86_64"
        ));
        assert!(PackageId::Zapret.matches_asset("zapret-v72.13.zip", w, "x86_64"));
        assert!(!PackageId::Zapret.matches_asset(
            "zapret-v72.13-openwrt-embedded.tar.gz",
            w,
            "x86_64"
        ));
        assert!(!PackageId::Zapret.matches_asset("sha256sum.txt", w, "x86_64"));
        assert!(PackageId::GoodbyeDpi.matches_asset("goodbyedpi-0.2.2.zip", w, "x86_64"));
    }

    #[test]
    fn matches_spoofdpi_assets() {
        let s = PackageId::SpoofDpi;
        assert!(s.matches_asset("spoofdpi_1.5.4_linux_x86_64.tar.gz", Os::Linux, "x86_64"));
        assert!(s.matches_asset("spoofdpi_1.5.4_linux_arm64.tar.gz", Os::Linux, "aarch64"));
        assert!(s.matches_asset("spoofdpi_1.5.4_darwin_arm64.tar.gz", Os::MacOs, "aarch64"));
        assert!(!s.matches_asset("spoofdpi_1.5.4_linux_x86_64.tar.gz", Os::MacOs, "x86_64"));
        assert!(!s.matches_asset("spoofdpi_1.5.4_linux_mips64.tar.gz", Os::Linux, "x86_64"));
        assert!(!s.matches_asset("spoofdpi_1.5.4_amd64.deb", Os::Linux, "x86_64"));
        assert!(!s.matches_asset("checksums.txt", Os::Linux, "x86_64"));
        assert!(!s.matches_asset("spoofdpi_1.5.4_linux_x86_64.tar.gz", Os::Windows, "x86_64"));
    }
}
