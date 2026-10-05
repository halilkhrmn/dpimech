use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Os {
    Windows,
    Linux,
    MacOs,
}

impl Os {
    pub const fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Os::Windows => "Windows",
            Os::Linux => "Linux",
            Os::MacOs => "macOS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingMode {
    /// Only selected applications are routed through the engine (e.g. ProxiFyre on Windows).
    PerApp,
    /// All matching traffic on the machine (WinDivert / NFQUEUE / pf).
    SystemWide,
    /// Only expose a local proxy port; the user configures apps manually.
    LocalProxy,
    /// A local proxy port that DPIMech hands to one app through its command line when it opens
    /// it (Chromium and Electron apps such as Discord: `--proxy-server`). No packet driver.
    AppProxy,
}

impl RoutingMode {
    pub const fn display_name(self) -> &'static str {
        match self {
            RoutingMode::PerApp => "Per-app",
            RoutingMode::SystemWide => "System-wide",
            RoutingMode::LocalProxy => "Local proxy",
            RoutingMode::AppProxy => "Open with proxy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    ByeDpi,
    ZapretWinws,
    ZapretNfqws,
    ZapretTpws,
    GoodbyeDpi,
    SpoofDpi,
}

impl EngineKind {
    pub const ALL: [EngineKind; 6] = [
        EngineKind::ByeDpi,
        EngineKind::ZapretWinws,
        EngineKind::ZapretNfqws,
        EngineKind::ZapretTpws,
        EngineKind::GoodbyeDpi,
        EngineKind::SpoofDpi,
    ];

    pub const fn display_name(self) -> &'static str {
        match self {
            EngineKind::ByeDpi => "ByeDPI",
            EngineKind::ZapretWinws => "zapret (winws)",
            EngineKind::ZapretNfqws => "zapret (nfqws)",
            EngineKind::ZapretTpws => "zapret (tpws)",
            EngineKind::GoodbyeDpi => "GoodbyeDPI",
            EngineKind::SpoofDpi => "SpoofDPI",
        }
    }

    /// Directory name under `<data>/engines/`.
    pub const fn slug(self) -> &'static str {
        match self {
            EngineKind::ByeDpi => "byedpi",
            EngineKind::ZapretWinws => "zapret-winws",
            EngineKind::ZapretNfqws => "zapret-nfqws",
            EngineKind::ZapretTpws => "zapret-tpws",
            EngineKind::GoodbyeDpi => "goodbyedpi",
            EngineKind::SpoofDpi => "spoofdpi",
        }
    }

    /// Executable file name without extension.
    pub const fn binary_stem(self) -> &'static str {
        match self {
            EngineKind::ByeDpi => "ciadpi",
            EngineKind::ZapretWinws => "winws",
            EngineKind::ZapretNfqws => "nfqws",
            EngineKind::ZapretTpws => "tpws",
            EngineKind::GoodbyeDpi => "goodbyedpi",
            EngineKind::SpoofDpi => "spoofdpi",
        }
    }

    pub const fn supported_os(self) -> &'static [Os] {
        match self {
            EngineKind::ByeDpi => &[Os::Windows, Os::Linux, Os::MacOs],
            EngineKind::ZapretWinws => &[Os::Windows],
            EngineKind::ZapretNfqws => &[Os::Linux],
            EngineKind::ZapretTpws => &[Os::Linux, Os::MacOs],
            EngineKind::GoodbyeDpi => &[Os::Windows],
            EngineKind::SpoofDpi => &[Os::Linux, Os::MacOs],
        }
    }

    /// Routing modes this engine can serve on the given OS.
    pub fn routing_modes(self, os: Os) -> Vec<RoutingMode> {
        use RoutingMode::*;
        if !self.supported_os().contains(&os) {
            return Vec::new();
        }
        match self {
            // Proxy engines: per-app needs a redirector (ProxiFyre on Windows, cgroups on Linux).
            // Per-app: ProxiFyre on Windows, cgroups + nftables on Linux.
            EngineKind::ByeDpi | EngineKind::SpoofDpi => match os {
                Os::Windows | Os::Linux => vec![PerApp, LocalProxy, AppProxy],
                Os::MacOs => vec![LocalProxy, AppProxy],
            },
            // "Open with proxy" is last everywhere: an option for people who want it, never offered.
            // Linux: whole computer through the same nftables redirect.
            EngineKind::ZapretTpws => match os {
                Os::Linux => vec![SystemWide, PerApp, LocalProxy, AppProxy],
                _ => vec![LocalProxy, AppProxy],
            },
            EngineKind::ZapretWinws | EngineKind::ZapretNfqws | EngineKind::GoodbyeDpi => {
                vec![SystemWide]
            }
        }
    }

    /// Output lines that mean the engine is alive but no longer serving new connections.
    pub const fn stall_markers(self) -> &'static [&'static str] {
        match self {
            // conev.c: the event pool reached --max-conn; new connections stall until restart.
            EngineKind::ByeDpi => &["pool is full"],
            _ => &[],
        }
    }

    /// Engines that are a local SOCKS5 proxy (other modes are built around that port).
    pub const fn is_proxy(self) -> bool {
        matches!(
            self,
            EngineKind::ByeDpi | EngineKind::ZapretTpws | EngineKind::SpoofDpi
        )
    }

    /// Engines that intercept packets (WinDivert on Windows, NFQUEUE on Linux) would both
    /// rewrite the same traffic, so only one may run at a time.
    pub const fn intercepts_packets(self) -> bool {
        matches!(
            self,
            EngineKind::ZapretWinws | EngineKind::GoodbyeDpi | EngineKind::ZapretNfqws
        )
    }

    pub fn available_on(os: Os) -> impl Iterator<Item = EngineKind> {
        Self::ALL
            .into_iter()
            .filter(move |e| e.supported_os().contains(&os))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Routing {
    PerApp {
        apps: Vec<String>,
        port: u16,
        /// Route only TCP; UDP (e.g. voice) goes direct and never occupies engine slots.
        #[serde(default)]
        tcp_only: bool,
    },
    SystemWide {
        /// Domains the engine should act on (written to the `{hostlist}` file).
        #[serde(default)]
        domains: Vec<String>,
    },
    LocalProxy {
        port: u16,
    },
    AppProxy {
        /// What the GUI opens: an executable, a command, a `.desktop` entry or a macOS `.app`.
        app: String,
        port: u16,
    },
}

impl Routing {
    pub fn mode(&self) -> RoutingMode {
        match self {
            Routing::PerApp { .. } => RoutingMode::PerApp,
            Routing::SystemWide { .. } => RoutingMode::SystemWide,
            Routing::LocalProxy { .. } => RoutingMode::LocalProxy,
            Routing::AppProxy { .. } => RoutingMode::AppProxy,
        }
    }

    pub fn port(&self) -> Option<u16> {
        match self {
            Routing::PerApp { port, .. }
            | Routing::LocalProxy { port }
            | Routing::AppProxy { port, .. } => Some(*port),
            Routing::SystemWide { .. } => None,
        }
    }
}

pub const DEFAULT_PROXY_PORT: u16 = 1080;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub engine: EngineKind,
    /// Raw engine arguments as the user sees them (strategy).
    pub args: String,
    pub routing: Routing,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub reliability: Reliability,
    /// Sites the connection monitor requests through the engine (e.g. `discord.com`).
    /// System-wide profiles fall back to their domain list.
    #[serde(default)]
    pub check_sites: Vec<String>,
}

impl Profile {
    /// Sites the connection monitor should request.
    pub fn monitored_sites(&self) -> Vec<String> {
        if !self.check_sites.is_empty() {
            return self.check_sites.clone();
        }
        match &self.routing {
            Routing::SystemWide { domains } => domains.iter().take(4).cloned().collect(),
            _ => Vec::new(),
        }
    }
}

/// What the connection monitor does when the profile's sites get slow or stop answering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlowAction {
    #[default]
    Restart,
    Warn,
}

/// How the service keeps a long-running engine healthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reliability {
    /// Restart the engine when it crashes or reports that it is stalled
    /// (e.g. ByeDPI's "pool is full" after hours of use).
    #[serde(default = "default_true")]
    pub auto_restart: bool,
    /// Preventive restart every N hours; 0 disables it.
    #[serde(default)]
    pub restart_every_hours: u32,
    /// Connection check interval in minutes; 0 disables the monitor.
    #[serde(default = "default_check_minutes")]
    pub check_every_minutes: u32,
    #[serde(default)]
    pub on_slow: SlowAction,
}

impl Default for Reliability {
    fn default() -> Self {
        Self {
            auto_restart: true,
            restart_every_hours: 0,
            check_every_minutes: default_check_minutes(),
            on_slow: SlowAction::Restart,
        }
    }
}

fn default_check_minutes() -> u32 {
    5
}

/// Latest result of the connection monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionHealth {
    /// Median response time of the sites that answered.
    pub latency_ms: u32,
    /// What this connection usually does (median of recent healthy checks); 0 while learning.
    pub usual_ms: u32,
    pub ok: u32,
    pub total: u32,
    pub checked_unix: u64,
    pub slow: bool,
    /// Set when automatic restarts did not help.
    #[serde(default)]
    pub advice: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Profile {
    pub fn new_id() -> String {
        uuid::Uuid::new_v4().simple().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProfileStatus {
    Stopped,
    Starting,
    Running {
        since_unix: u64,
        /// Automatic restarts since the user started the profile.
        #[serde(default)]
        restarts: u32,
        #[serde(default)]
        health: Option<ConnectionHealth>,
    },
    Error {
        message: String,
    },
}

impl ProfileStatus {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ProfileStatus::Starting | ProfileStatus::Running { .. }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileState {
    pub profile: Profile,
    pub status: ProfileStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    /// Only sent while a GUI has asked for the detailed log (`SetDetailedLog`), so older GUIs
    /// that do not know this level never receive it.
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub unix_ms: u64,
    pub level: LogLevel,
    /// "service" or a profile name.
    pub source: String,
    pub text: String,
}
