//! Per-app and system-wide routing on Linux, the counterpart of ProxiFyre on Windows.
//!
//! * The processes of the chosen apps (matched by executable name) are moved into a cgroup
//!   per profile, `dpimech/app-<port>`; children they start later inherit it.
//! * An nftables `nat output` rule sends new TCP connections from that cgroup to a relay in
//!   this service (`socket cgroupv2` match + `redirect`).
//! * The relay reads the original destination (`SO_ORIGINAL_DST`) and connects there through
//!   the engine's local SOCKS5 port, so ByeDPI and tpws run unchanged.
//! * A "system-wide" route (tpws for the whole computer) redirects all outgoing TCP to the
//!   chosen ports instead, except the service's own cgroup so engines never loop into
//!   themselves.
//!
//! Only TCP is routed; UDP (e.g. voice) goes direct. Local and private addresses are never
//! redirected. Everything is undone when the routes change or the service stops.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use anyhow::{Context, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;

use crate::logs::LogBus;
use crate::packages::Packages;
use crate::perapp::Route;

const SOURCE: &str = "routing";
const TABLE: &str = "dpimech_route";
/// How often new processes of the chosen apps are looked for.
const SCAN_EVERY: Duration = Duration::from_secs(1);
/// When to point out that none of a profile's apps is running.
const NOTHING_MATCHED_AFTER: Duration = Duration::from_secs(20);

struct Active {
    routes: Vec<Route>,
    tasks: Vec<JoinHandle<()>>,
    cgroups: Cgroups,
    /// pid → cgroup it came from (relative to the cgroup2 root), to put it back afterwards.
    moved: Arc<StdMutex<HashMap<u32, String>>>,
}

pub struct PerAppRouter {
    logs: Arc<LogBus>,
    active: Mutex<Option<Active>>,
    /// Receives a message when routing breaks down on its own.
    failures: mpsc::UnboundedSender<String>,
}

impl PerAppRouter {
    pub fn new(
        _packages: Packages,
        logs: Arc<LogBus>,
        failures: mpsc::UnboundedSender<String>,
    ) -> Self {
        Self {
            logs,
            active: Mutex::new(None),
            failures,
        }
    }

    /// Makes the routing match exactly `routes`. An empty list removes everything.
    pub async fn apply(&self, mut routes: Vec<Route>) -> anyhow::Result<()> {
        routes.sort_by_key(|r| r.port);
        let mut active = self.active.lock().await;
        if active.as_ref().map(|a| &a.routes) == Some(&routes) {
            return Ok(());
        }
        if let Some(old) = active.take() {
            teardown(old);
        }
        if routes.is_empty() {
            return Ok(());
        }
        let started = self.start(&routes).await;
        match started {
            Ok(new) => {
                self.logs.info(
                    SOURCE,
                    format!("routing {} profile(s) through their engines", routes.len()),
                );
                *active = Some(Active { routes, ..new });
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn start(&self, routes: &[Route]) -> anyhow::Result<Active> {
        let cgroups = Cgroups::find()?;
        let moved: Arc<StdMutex<HashMap<u32, String>>> = Arc::default();
        let mut tasks = Vec::new();
        let mut rules = Vec::new();
        let mut targets = Vec::new();

        for route in routes {
            let v4 = TcpListener::bind("127.0.0.1:0")
                .await
                .context("starting the routing relay")?;
            // IPv6 is optional: some systems have no ::1.
            let v6 = TcpListener::bind("[::1]:0").await.ok();
            let redirect = Redirect {
                v4: v4.local_addr()?.port(),
                v6: v6
                    .as_ref()
                    .and_then(|l| l.local_addr().ok())
                    .map(|a| a.port()),
            };
            for listener in std::iter::once(v4).chain(v6) {
                tasks.push(tokio::spawn(relay(
                    listener,
                    route.port,
                    self.logs.clone(),
                    self.failures.clone(),
                )));
            }
            match &route.system_wide {
                Some(ports) => rules.push(RuleTarget::All {
                    ports: ports.clone(),
                    redirect,
                }),
                None => {
                    let name = format!("dpimech/app-{}", route.port);
                    cgroups.create(&name)?;
                    targets.push((
                        route.apps.iter().map(|a| app_key(a)).collect::<Vec<_>>(),
                        name.clone(),
                    ));
                    rules.push(RuleTarget::Apps {
                        cgroup: name,
                        redirect,
                    });
                }
            }
        }

        // Engines (children of this service) must never be redirected into themselves.
        let own = cgroups.own_for_exclusion()?;
        let script = ruleset(&cgroups.nft_prefix, own.as_deref(), &rules);
        if let Err(e) = crate::nfqueue::nft(&script) {
            for task in &tasks {
                task.abort();
            }
            cgroups.remove_all();
            bail!("installing the nftables routing rules: {e}");
        }

        if !targets.is_empty() {
            let cgroups = cgroups.clone();
            let moved = moved.clone();
            let logs = self.logs.clone();
            tasks.push(tokio::spawn(async move {
                let started = std::time::Instant::now();
                let mut warned = false;
                let wanted: Vec<String> = targets.iter().flat_map(|(apps, _)| apps.clone()).collect();
                loop {
                    let (cgroups, targets_now, moved_now) =
                        (cgroups.clone(), targets.clone(), moved.clone());
                    let newly = tokio::task::spawn_blocking(move || {
                        move_matching(&cgroups, &targets_now, &moved_now)
                    })
                    .await
                    .unwrap_or_default();
                    if !newly.is_empty() {
                        logs.info(SOURCE, format!("now routed: {}", describe_moved(&newly)));
                    }
                    // An app that never shows up usually runs under another process name.
                    if !warned
                        && started.elapsed() > NOTHING_MATCHED_AFTER
                        && moved.lock().unwrap().is_empty()
                    {
                        warned = true;
                        logs.warn(
                            SOURCE,
                            format!(
                                "no running process matches {} yet; start the app, or check its name in the profile",
                                wanted.join(", ")
                            ),
                        );
                    }
                    tokio::time::sleep(SCAN_EVERY).await;
                }
            }));
        }

        Ok(Active {
            routes: Vec::new(),
            tasks,
            cgroups,
            moved,
        })
    }
}

impl Drop for PerAppRouter {
    fn drop(&mut self) {
        if let Some(active) = self.active.get_mut().take() {
            teardown(active);
        }
    }
}

fn teardown(active: Active) {
    for task in &active.tasks {
        task.abort();
    }
    let _ = crate::nfqueue::nft(&format!("destroy table inet {TABLE}\n"))
        .or_else(|_| crate::nfqueue::nft(&format!("delete table inet {TABLE}\n")));
    // Put every process back where it was; children started inside follow their parent's
    // original cgroup, or the root when that is unknown.
    let moved = active.moved.lock().unwrap().clone();
    let fallback = moved.values().next().cloned().unwrap_or_default();
    for cgroup in active.cgroups.created() {
        for pid in active.cgroups.procs(&cgroup) {
            let back = moved.get(&pid).unwrap_or(&fallback);
            active.cgroups.move_pid(pid, back);
        }
    }
    active.cgroups.remove_all();
}

/// Removes the table a crashed service may have left behind (called at start).
pub fn remove_leftovers() {
    let _ = crate::nfqueue::nft(&format!("destroy table inet {TABLE}\n"))
        .or_else(|_| crate::nfqueue::nft(&format!("delete table inet {TABLE}\n")));
}

#[derive(Debug, Clone, Copy)]
struct Redirect {
    v4: u16,
    v6: Option<u16>,
}

enum RuleTarget {
    Apps { cgroup: String, redirect: Redirect },
    All { ports: String, redirect: Redirect },
}

fn ruleset(prefix: &str, own: Option<&str>, targets: &[RuleTarget]) -> String {
    let mut rules = String::new();
    // Per-app rules first, so a chosen app keeps its own engine even with a system-wide route.
    let mut ordered: Vec<&RuleTarget> = targets
        .iter()
        .filter(|t| matches!(t, RuleTarget::Apps { .. }))
        .collect();
    ordered.extend(
        targets
            .iter()
            .filter(|t| matches!(t, RuleTarget::All { .. })),
    );
    for target in ordered {
        let (matcher, redirect) = match target {
            RuleTarget::Apps { cgroup, redirect } => (
                format!(
                    "socket cgroupv2 level {} \"{prefix}{cgroup}\" ",
                    cgroup.split('/').count()
                ),
                redirect,
            ),
            RuleTarget::All { ports, redirect } => (format!("tcp dport {{ {ports} }} "), redirect),
        };
        rules.push_str(&format!(
            "        meta nfproto ipv4 meta l4proto tcp {matcher}redirect to :{}\n",
            redirect.v4
        ));
        if let Some(v6) = redirect.v6 {
            rules.push_str(&format!(
                "        meta nfproto ipv6 meta l4proto tcp {matcher}redirect to :{v6}\n"
            ));
        }
    }
    let own_rule = own
        .map(|own| {
            format!(
                "        socket cgroupv2 level {} \"{prefix}{own}\" return\n",
                own.split('/').count()
            )
        })
        .unwrap_or_default();
    format!(
        "\
table inet {TABLE} {{
    chain out {{
        type nat hook output priority -100; policy accept;
        ip daddr {{ 127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 169.254.0.0/16 }} return
        ip6 daddr {{ ::1, fc00::/7, fe80::/10 }} return
{own_rule}{rules}    }}
}}
"
    )
}

/// App names are matched like ProxiFyre does on Windows: by executable name, case-insensitive,
/// without an `.exe` suffix (profiles may come from Windows).
fn app_key(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_owned()
}

/// Moves every process of a chosen app into its profile's cgroup; returns the processes it
/// moved this time as (name, pid).
fn move_matching(
    cgroups: &Cgroups,
    targets: &[(Vec<String>, String)],
    moved: &StdMutex<HashMap<u32, String>>,
) -> Vec<(String, u32)> {
    let me = std::process::id();
    let mut newly = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return newly;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == me {
            continue;
        }
        let dir = entry.path();
        let names = process_names(&dir);
        let Some((apps, target)) = targets
            .iter()
            .find(|(apps, _)| names.iter().any(|n| apps.contains(n)))
        else {
            continue;
        };
        let Some(current) = current_cgroup(&dir) else {
            continue;
        };
        if &current == target {
            continue;
        }
        if cgroups.move_pid(pid, target) {
            moved.lock().unwrap().entry(pid).or_insert(current);
            let name = names
                .iter()
                .find(|n| apps.contains(n))
                .cloned()
                .unwrap_or_default();
            newly.push((name, pid));
        }
    }
    newly
}

/// "discord (pids 101, 102, 103)"; many processes of one app are grouped.
fn describe_moved(moved: &[(String, u32)]) -> String {
    let mut by_name: std::collections::BTreeMap<&str, Vec<u32>> = Default::default();
    for (name, pid) in moved {
        by_name.entry(name).or_default().push(*pid);
    }
    by_name
        .into_iter()
        .map(|(name, pids)| {
            let list: Vec<String> = pids.iter().take(8).map(u32::to_string).collect();
            let more = if pids.len() > 8 { ", …" } else { "" };
            format!("{name} (pid {}{more})", list.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Executable file name and `comm`, lower-case: Electron apps often run as `electron` with
/// the app's name in `comm`, or the other way round.
fn process_names(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(exe) = std::fs::read_link(dir.join("exe"))
        && let Some(name) = exe.file_name()
    {
        names.push(app_key(&name.to_string_lossy()));
    }
    if let Ok(comm) = std::fs::read_to_string(dir.join("comm")) {
        names.push(app_key(&comm));
    }
    names
}

/// The process's cgroup v2 path without the leading `/`.
fn current_cgroup(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join("cgroup"))
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("0::"))
        .map(|p| p.trim_start_matches('/').to_owned())
}

/// The cgroup v2 hierarchy: `/sys/fs/cgroup` on current systems, `/sys/fs/cgroup/unified`
/// on older "hybrid" ones. nftables resolves cgroup names relative to `/sys/fs/cgroup`, so
/// the hybrid layout needs the `unified/` prefix in rules.
#[derive(Clone)]
struct Cgroups {
    root: PathBuf,
    nft_prefix: String,
    created: Arc<StdMutex<Vec<String>>>,
}

impl Cgroups {
    fn find() -> anyhow::Result<Cgroups> {
        for (root, prefix) in [
            ("/sys/fs/cgroup", ""),
            ("/sys/fs/cgroup/unified", "unified/"),
        ] {
            if Path::new(root).join("cgroup.controllers").exists() {
                return Ok(Cgroups {
                    root: root.into(),
                    nft_prefix: prefix.into(),
                    created: Arc::default(),
                });
            }
        }
        bail!("per-app routing needs cgroup v2 (not found under /sys/fs/cgroup)")
    }

    fn create(&self, name: &str) -> anyhow::Result<()> {
        let path = self.root.join(name);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("creating cgroup {}", path.display()))?;
        let mut created = self.created.lock().unwrap();
        if !created.iter().any(|c| c == name) {
            created.push(name.to_owned());
        }
        Ok(())
    }

    fn created(&self) -> Vec<String> {
        self.created.lock().unwrap().clone()
    }

    fn procs(&self, name: &str) -> Vec<u32> {
        std::fs::read_to_string(self.root.join(name).join("cgroup.procs"))
            .map(|t| t.lines().filter_map(|l| l.trim().parse().ok()).collect())
            .unwrap_or_default()
    }

    fn move_pid(&self, pid: u32, to: &str) -> bool {
        std::fs::write(self.root.join(to).join("cgroup.procs"), pid.to_string()).is_ok()
    }

    /// Where this service runs, for the rule that keeps engines out of the redirect. Outside
    /// systemd (e.g. in a container) the service may sit in the root cgroup, which cannot be
    /// matched; then it moves itself and its engines into `dpimech/service`.
    fn own_for_exclusion(&self) -> anyhow::Result<Option<String>> {
        let own = current_cgroup(Path::new("/proc/self")).unwrap_or_default();
        if !own.is_empty() {
            return Ok(Some(own));
        }
        let name = "dpimech/service";
        self.create(name)?;
        let me = std::process::id();
        self.move_pid(me, name);
        for pid in children_of(me) {
            self.move_pid(pid, name);
        }
        Ok(Some(name.to_owned()))
    }

    /// Removes the cgroups this router created (deepest first); non-empty ones stay.
    fn remove_all(&self) {
        let mut created = self.created();
        created.sort_by_key(|c| std::cmp::Reverse(c.matches('/').count()));
        for name in created {
            if name == "dpimech/service" {
                continue; // the service itself lives there
            }
            let _ = std::fs::remove_dir(self.root.join(&name));
        }
        let _ = std::fs::remove_dir(self.root.join("dpimech"));
    }
}

fn children_of(parent: u32) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()))
        .filter(|pid| {
            std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .ok()
                .and_then(|s| {
                    // "pid (comm) state ppid ..."; comm may contain spaces, so split after ')'.
                    s.rsplit_once(')')
                        .and_then(|(_, rest)| rest.split_whitespace().nth(1)?.parse::<u32>().ok())
                })
                == Some(parent)
        })
        .collect()
}

async fn relay(
    listener: TcpListener,
    socks_port: u16,
    logs: Arc<LogBus>,
    failures: mpsc::UnboundedSender<String>,
) {
    loop {
        match listener.accept().await {
            Ok((inbound, _)) => {
                let logs = logs.clone();
                tokio::spawn(async move {
                    if let Err(e) = forward(inbound, socks_port, &logs).await {
                        tracing::debug!("relay connection: {e}");
                        if e.kind() == std::io::ErrorKind::ConnectionRefused {
                            logs.push(
                                dpimech_core::model::LogLevel::Warn,
                                SOURCE,
                                format!("engine on port {socks_port} is not answering"),
                            );
                        }
                    }
                });
            }
            Err(e) => {
                let _ = failures.send(format!("routing relay stopped: {e}"));
                return;
            }
        }
    }
}

/// A stuck engine must not keep the app's connections hanging forever.
const ENGINE_HANDSHAKE: Duration = Duration::from_secs(15);

/// Opens the SOCKS5 tunnel to `target` through the engine.
async fn open_tunnel(socks_port: u16, target: SocketAddr) -> std::io::Result<TcpStream> {
    tokio::time::timeout(ENGINE_HANDSHAKE, async {
        let mut outbound = TcpStream::connect(("127.0.0.1", socks_port)).await?;
        socks5_connect(&mut outbound, target).await?;
        Ok(outbound)
    })
    .await
    .unwrap_or_else(|_| {
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "the engine did not answer",
        ))
    })
}

async fn forward(mut inbound: TcpStream, socks_port: u16, logs: &LogBus) -> std::io::Result<()> {
    let target = original_destination(&inbound)?;
    if !logs.detailed() {
        let mut outbound = open_tunnel(socks_port, target).await?;
        tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await?;
        return Ok(());
    }
    // Detailed log: name the site (TLS SNI) and say how the connection ended. peek() leaves
    // the bytes in place; apps where the server talks first just get no name.
    let started = std::time::Instant::now();
    let mut first = [0u8; 2048];
    let name =
        match tokio::time::timeout(Duration::from_millis(300), inbound.peek(&mut first)).await {
            Ok(Ok(n)) => crate::sni::server_name(&first[..n]),
            _ => None,
        };
    let what = match &name {
        Some(name) => format!("{name} ({target})"),
        None => target.to_string(),
    };
    let result = async {
        let mut outbound = open_tunnel(socks_port, target).await?;
        tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await
    }
    .await;
    let secs = started.elapsed().as_secs_f32();
    logs.debug(SOURCE, || match &result {
        Ok((up, down)) => {
            format!("{what}: sent {up} B, received {down} B, closed after {secs:.1} s")
        }
        Err(e) => format!("{what}: {e} after {secs:.1} s"),
    });
    result.map(drop)
}

/// Where a redirected connection was originally going (netfilter's `SO_ORIGINAL_DST`).
fn original_destination(stream: &TcpStream) -> std::io::Result<SocketAddr> {
    // SO_ORIGINAL_DST and IP6T_SO_ORIGINAL_DST share the value 80.
    const SO_ORIGINAL_DST: libc::c_int = 80;
    let fd = stream.as_raw_fd();
    if stream.local_addr()?.is_ipv4() {
        // SAFETY: `addr` is a correctly sized sockaddr_in and `len` holds its size.
        let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        let mut len = size_of::<libc::sockaddr_in>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_IP,
                SO_ORIGINAL_DST,
                (&mut addr as *mut libc::sockaddr_in).cast(),
                &mut len,
            )
        };
        if rc != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr)),
            u16::from_be(addr.sin_port),
        )))
    } else {
        // SAFETY: as above, for sockaddr_in6.
        let mut addr: libc::sockaddr_in6 = unsafe { std::mem::zeroed() };
        let mut len = size_of::<libc::sockaddr_in6>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_IPV6,
                SO_ORIGINAL_DST,
                (&mut addr as *mut libc::sockaddr_in6).cast(),
                &mut len,
            )
        };
        if rc != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::from(addr.sin6_addr.s6_addr),
            u16::from_be(addr.sin6_port),
            0,
            0,
        )))
    }
}

/// SOCKS5 CONNECT without authentication, by IP address (the engine sees the TLS SNI in the
/// stream, which is what it works on).
async fn socks5_connect(stream: &mut TcpStream, target: SocketAddr) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};

    stream.write_all(&[5, 1, 0]).await?;
    let mut choice = [0u8; 2];
    stream.read_exact(&mut choice).await?;
    if choice != [5, 0] {
        return Err(Error::other("SOCKS5 greeting refused"));
    }
    let mut request = vec![5, 1, 0];
    match target {
        SocketAddr::V4(a) => {
            request.push(1);
            request.extend_from_slice(&a.ip().octets());
        }
        SocketAddr::V6(a) => {
            request.push(4);
            request.extend_from_slice(&a.ip().octets());
        }
    }
    request.extend_from_slice(&target.port().to_be_bytes());
    stream.write_all(&request).await?;
    let mut head = [0u8; 4];
    stream.read_exact(&mut head).await?;
    if head[1] != 0 {
        return Err(Error::new(
            ErrorKind::ConnectionRefused,
            format!("SOCKS5 connect failed ({})", head[1]),
        ));
    }
    let rest = match head[3] {
        1 => 4 + 2,
        4 => 16 + 2,
        3 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            usize::from(len[0]) + 2
        }
        _ => return Err(Error::new(ErrorKind::InvalidData, "bad SOCKS5 reply")),
    };
    let mut skip = vec![0u8; rest];
    stream.read_exact(&mut skip).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruleset_orders_app_rules_before_system_wide_and_excludes_the_service() {
        let r = Redirect {
            v4: 40001,
            v6: Some(40002),
        };
        let text = ruleset(
            "unified/",
            Some("system.slice/dpimech.service"),
            &[
                RuleTarget::All {
                    ports: "80, 443".into(),
                    redirect: r,
                },
                RuleTarget::Apps {
                    cgroup: "dpimech/app-1080".into(),
                    redirect: r,
                },
            ],
        );
        let own = text
            .find("socket cgroupv2 level 2 \"unified/system.slice/dpimech.service\" return")
            .unwrap();
        let app = text
            .find("meta nfproto ipv4 meta l4proto tcp socket cgroupv2 level 2 \"unified/dpimech/app-1080\" redirect to :40001")
            .unwrap();
        let all = text
            .find("meta nfproto ipv4 meta l4proto tcp tcp dport { 80, 443 } redirect to :40001")
            .unwrap();
        assert!(own < app && app < all, "{text}");
        assert!(text.contains(
            "meta nfproto ipv6 meta l4proto tcp tcp dport { 80, 443 } redirect to :40002"
        ));
        assert!(text.contains("ip daddr { 127.0.0.0/8"));
    }

    #[test]
    fn app_names_match_like_proxifyre() {
        assert_eq!(app_key("Discord.exe"), "discord");
        assert_eq!(app_key(" Discord\n"), "discord");
    }
}
