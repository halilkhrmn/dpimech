//! The DNS servers the computer uses right now, per connected network, so DPIMech can show them
//! (a wrong or blocking resolver explains many "still blocked" cases, see dnscheck.rs), and the
//! optional switch to a well-known public DNS service.
//!
//! Setting DNS is a system change made with the service's rights for any local user, so only the
//! services in `catalog::PUBLIC_DNS` can be chosen (never an address from the request), the
//! previous settings are saved before the first change, and "the network's own" or uninstalling
//! DPIMech puts them back.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use dpimech_core::catalog::PUBLIC_DNS;
use dpimech_core::model::DnsInUse;
use dpimech_core::paths::DataDir;
use serde::{Deserialize, Serialize};

/// Connected networks and their DNS servers. Reading only; nothing is changed here.
pub fn in_use() -> Vec<DnsInUse> {
    platform::in_use()
}

/// What DPIMech changed: the chosen service and, per connection, what to put back.
#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    provider: Option<String>,
    #[serde(default)]
    saved: Vec<Saved>,
}

/// One connection's settings from before DPIMech changed them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Saved {
    /// Windows: the adapter's GUID; Linux: the link name.
    pub id: String,
    /// Hand-set servers before the change (Windows `NameServer`, IPv4 then IPv6); empty for
    /// "from the network".
    #[serde(default)]
    pub v4: String,
    #[serde(default)]
    pub v6: String,
}

fn state_path(data: &DataDir) -> PathBuf {
    data.root.join("dns.json")
}

fn load(path: &Path) -> State {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn store(path: &Path, state: &State) -> anyhow::Result<()> {
    if state.provider.is_none() && state.saved.is_empty() {
        let _ = std::fs::remove_file(path);
        return Ok(());
    }
    std::fs::write(path, serde_json::to_vec_pretty(state)?)
        .with_context(|| format!("writing {}", path.display()))
}

/// The public DNS service DPIMech set, if any.
pub fn chosen(data: &DataDir) -> Option<String> {
    load(&state_path(data)).provider
}

/// Addresses of a service in `PUBLIC_DNS` (IPv4, IPv6); `None` for any other name.
fn addresses(provider: &str) -> Option<(Vec<&'static str>, Vec<&'static str>)> {
    let (_, all) = PUBLIC_DNS.iter().find(|(name, _)| *name == provider)?;
    Some(all.iter().partition(|a| !a.contains(':')))
}

/// Switches every connected network to `provider`, or back to its own settings with `None`.
pub fn set(data: &DataDir, provider: Option<&str>) -> anyhow::Result<()> {
    let path = state_path(data);
    let mut state = load(&path);
    match provider {
        None => {
            platform::restore(&state.saved)?;
            state = State::default();
        }
        Some(name) => {
            let Some((v4, v6)) = addresses(name) else {
                bail!("unknown DNS service \"{name}\"");
            };
            platform::apply(&v4, &v6, &mut state.saved, false)?;
            state.provider = Some(name.to_owned());
        }
    }
    store(&path, &state)
}

/// Keeps the chosen service on connections that came up since (another Wi-Fi, a cable): called
/// at service start and now and then. Does nothing when DPIMech set no DNS.
pub fn keep(data: &DataDir) {
    let path = state_path(data);
    let mut state = load(&path);
    let Some((v4, v6)) = state.provider.as_deref().and_then(addresses) else {
        return;
    };
    // Only connections DPIMech has not changed yet: re-applying to the others every time would
    // fight, e.g., an IPv6 server the router keeps announcing.
    let before = state.saved.len();
    let result = platform::apply(&v4, &v6, &mut state.saved, true);
    if state.saved.len() == before && result.is_ok() {
        return;
    }
    match result {
        Ok(()) => {
            let _ = store(&path, &state);
        }
        Err(e) => tracing::warn!("keeping the chosen DNS: {e:#}"),
    }
}

/// Uninstall: puts every connection's own DNS back.
pub fn restore_on_uninstall(data: &DataDir) {
    let path = state_path(data);
    let state = load(&path);
    if state.provider.is_none() && state.saved.is_empty() {
        return;
    }
    match platform::restore(&state.saved) {
        Ok(()) => {
            let _ = std::fs::remove_file(&path);
            println!("DNS settings put back");
        }
        Err(e) => eprintln!("could not put the DNS settings back: {e:#}"),
    }
}

/// Runs a system tool; its error output becomes the error.
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn run(program: &str, args: &[&str]) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().with_context(|| format!("running {program}"))?;
    if !out.status.success() {
        let mut why = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        if why.is_empty() {
            why = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        }
        bail!("{program} {}: {why}", args.join(" "));
    }
    Ok(())
}

#[cfg(windows)]
mod platform {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use dpimech_core::model::DnsInUse;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_MULTICAST,
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
    };

    const ERROR_BUFFER_OVERFLOW: u32 = 111;
    const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;

    use super::{Saved, run};

    /// One network adapter as DNS settings see it.
    struct Adapter {
        name: String,
        /// `{GUID}`: stays the same across restarts, unlike the index.
        id: String,
        if_index: u32,
        ipv6_if_index: u32,
        servers: Vec<String>,
        /// Up with a gateway: it carries traffic.
        connected: bool,
    }

    pub fn in_use() -> Vec<DnsInUse> {
        adapters()
            .into_iter()
            .filter(|a| a.connected && !a.servers.is_empty())
            .map(|a| DnsInUse {
                manual: Some(set_by_hand(&a.id)),
                connection: a.name,
                servers: a.servers,
            })
            .collect()
    }

    pub fn apply(
        v4: &[&str],
        v6: &[&str],
        saved: &mut Vec<Saved>,
        only_new: bool,
    ) -> anyhow::Result<()> {
        let mut changed = 0;
        for a in adapters().into_iter().filter(|a| a.connected) {
            let known = saved.iter().any(|s| s.id == a.id);
            if only_new && known {
                continue;
            }
            if !known {
                saved.push(Saved {
                    id: a.id.clone(),
                    v4: name_server(&a.id, "Tcpip"),
                    v6: name_server(&a.id, "Tcpip6"),
                });
            }
            set_servers("ipv4", a.if_index, v4)?;
            // No IPv6 on this adapter (or switched off): its IPv4 servers are what counts.
            if a.ipv6_if_index != 0
                && let Err(e) = set_servers("ipv6", a.ipv6_if_index, v6)
            {
                tracing::debug!("DNS for IPv6 on {}: {e:#}", a.name);
            }
            changed += 1;
        }
        if changed > 0 {
            let _ = run("ipconfig", &["/flushdns"]);
        }
        Ok(())
    }

    pub fn restore(saved: &[Saved]) -> anyhow::Result<()> {
        let all = adapters();
        let mut first_error = None;
        for s in saved {
            // An adapter that is gone (unplugged USB Wi-Fi) has nothing to put back.
            let Some(a) = all.iter().find(|a| a.id == s.id) else {
                continue;
            };
            let v4: Vec<&str> = split_servers(&s.v4);
            let v6: Vec<&str> = split_servers(&s.v6);
            if let Err(e) = put_back("ipv4", a.if_index, &v4) {
                first_error.get_or_insert(e);
            }
            if a.ipv6_if_index != 0 {
                let _ = put_back("ipv6", a.ipv6_if_index, &v6);
            }
        }
        let _ = run("ipconfig", &["/flushdns"]);
        first_error.map_or(Ok(()), Err)
    }

    fn put_back(family: &str, index: u32, servers: &[&str]) -> anyhow::Result<()> {
        if servers.is_empty() {
            let name = format!("name={index}");
            run(
                "netsh",
                &[
                    "interface",
                    family,
                    "set",
                    "dnsservers",
                    &name,
                    "source=dhcp",
                ],
            )
        } else {
            set_servers(family, index, servers)
        }
    }

    /// `netsh interface <ipv4|ipv6> set dnsservers`: the first server replaces the list, the
    /// others are added in order. `name=` takes the interface index, which is not localised.
    fn set_servers(family: &str, index: u32, servers: &[&str]) -> anyhow::Result<()> {
        let name = format!("name={index}");
        for (i, server) in servers.iter().enumerate() {
            let address = format!("address={server}");
            if i == 0 {
                run(
                    "netsh",
                    &[
                        "interface",
                        family,
                        "set",
                        "dnsservers",
                        &name,
                        "source=static",
                        &address,
                        "register=primary",
                        "validate=no",
                    ],
                )?;
            } else {
                let position = format!("index={}", i + 1);
                run(
                    "netsh",
                    &[
                        "interface",
                        family,
                        "add",
                        "dnsservers",
                        &name,
                        &address,
                        &position,
                        "validate=no",
                    ],
                )?;
            }
        }
        Ok(())
    }

    /// Windows keeps hand-set servers comma or space separated.
    fn split_servers(text: &str) -> Vec<&str> {
        text.split([',', ' '])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn adapters() -> Vec<Adapter> {
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_INCLUDE_GATEWAYS;
        let mut size: u32 = 16 * 1024;
        let mut buffer: Vec<u64> = Vec::new();
        // The list can grow between the size query and the call: retry a few times.
        for _ in 0..3 {
            buffer = vec![0u64; (size as usize).div_ceil(8)];
            // SAFETY: the buffer is `size` bytes long, 8-byte aligned as the struct needs.
            let result = unsafe {
                GetAdaptersAddresses(
                    AF_UNSPEC as u32,
                    flags,
                    std::ptr::null(),
                    buffer.as_mut_ptr().cast(),
                    &mut size,
                )
            };
            match result {
                0 => break,
                ERROR_BUFFER_OVERFLOW => continue,
                _ => return Vec::new(),
            }
        }
        let mut out = Vec::new();
        let mut next = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        // SAFETY: a linked list inside `buffer`, filled by GetAdaptersAddresses.
        while let Some(adapter) = unsafe { next.as_ref() } {
            next = adapter.Next;
            if adapter.IfType == IF_TYPE_SOFTWARE_LOOPBACK {
                continue;
            }
            let connected =
                adapter.OperStatus == IfOperStatusUp && !adapter.FirstGatewayAddress.is_null();
            let mut servers = Vec::new();
            let mut dns = adapter.FirstDnsServerAddress;
            // SAFETY: same list; each sockaddr is as long as the adapter says.
            while let Some(entry) = unsafe { dns.as_ref() } {
                if let Some(ip) = unsafe { sockaddr_ip(entry.Address.lpSockaddr) } {
                    servers.push(ip.to_string());
                }
                dns = entry.Next;
            }
            // SAFETY: both are NUL-terminated strings owned by the buffer; the union holds
            // the index on every Windows version that has this struct.
            let (name, id, if_index) = unsafe {
                (
                    wide_to_string(adapter.FriendlyName),
                    ansi_to_string(adapter.AdapterName),
                    adapter.Anonymous1.Anonymous.IfIndex,
                )
            };
            out.push(Adapter {
                name,
                id,
                if_index,
                ipv6_if_index: adapter.Ipv6IfIndex,
                servers,
                connected,
            });
        }
        out
    }

    unsafe fn sockaddr_ip(addr: *const SOCKADDR) -> Option<IpAddr> {
        // SAFETY: the caller passes a sockaddr from GetAdaptersAddresses (or null).
        unsafe {
            let family = addr.as_ref()?.sa_family;
            if family == AF_INET {
                let v4 = &*(addr as *const SOCKADDR_IN);
                Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(
                    v4.sin_addr.S_un.S_addr,
                ))))
            } else if family == AF_INET6 {
                let v6 = &*(addr as *const SOCKADDR_IN6);
                Some(IpAddr::V6(Ipv6Addr::from(v6.sin6_addr.u.Byte)))
            } else {
                None
            }
        }
    }

    unsafe fn wide_to_string(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        // SAFETY: NUL-terminated UTF-16 from the system.
        unsafe {
            let len = (0..).take_while(|&i| *p.add(i) != 0).count();
            String::from_utf16_lossy(std::slice::from_raw_parts(p, len))
        }
    }

    unsafe fn ansi_to_string(p: *const u8) -> String {
        if p.is_null() {
            return String::new();
        }
        // SAFETY: NUL-terminated ASCII (the adapter's GUID) from the system.
        unsafe {
            std::ffi::CStr::from_ptr(p.cast())
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Windows keeps hand-set DNS servers in the interface's `NameServer` value; servers from
    /// DHCP or router advertisements go elsewhere.
    fn set_by_hand(adapter_id: &str) -> bool {
        ["Tcpip", "Tcpip6"]
            .iter()
            .any(|stack| !name_server(adapter_id, stack).is_empty())
    }

    /// The hand-set servers of one adapter for `Tcpip` (IPv4) or `Tcpip6`; empty when none.
    fn name_server(adapter_id: &str, stack: &str) -> String {
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(format!(
                r"SYSTEM\CurrentControlSet\Services\{stack}\Parameters\Interfaces\{adapter_id}"
            ))
            .and_then(|key| key.get_value::<String, _>("NameServer"))
            .map(|v| v.trim().to_owned())
            .unwrap_or_default()
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use dpimech_core::model::DnsInUse;

    /// systemd-resolved lists the real upstream servers here, while /etc/resolv.conf only names
    /// its local stub (127.0.0.53).
    const RESOLVED: &str = "/run/systemd/resolve/resolv.conf";

    pub fn in_use() -> Vec<DnsInUse> {
        let read = |p: &str| std::fs::read_to_string(p).ok();
        let etc = read("/etc/resolv.conf").unwrap_or_default();
        let mut servers = nameservers(&etc);
        if servers.iter().all(|s| s.starts_with("127.")) {
            if let Some(upstream) = read(RESOLVED).map(|t| nameservers(&t)) {
                if !upstream.is_empty() {
                    servers = upstream;
                }
            }
        }
        if servers.is_empty() {
            return Vec::new();
        }
        vec![DnsInUse {
            connection: "system".into(),
            servers,
            manual: None,
        }]
    }

    use super::{Saved, run};

    /// systemd-resolved: per link, at run time only. A restart of the computer or of the
    /// connection brings the network's own servers back; `keep` sets them again.
    pub fn apply(
        v4: &[&str],
        v6: &[&str],
        saved: &mut Vec<Saved>,
        only_new: bool,
    ) -> anyhow::Result<()> {
        if !std::path::Path::new("/run/systemd/resolve").is_dir() {
            anyhow::bail!(
                "this system does not use systemd-resolved, which DPIMech needs to set DNS; set it in your network settings instead"
            );
        }
        let servers: Vec<&str> = v4.iter().chain(v6).copied().collect();
        let mut changed = 0;
        for link in default_links() {
            let known = saved.iter().any(|s| s.id == link);
            if only_new && known {
                continue;
            }
            if !known {
                saved.push(Saved {
                    id: link.clone(),
                    v4: String::new(),
                    v6: String::new(),
                });
            }
            let mut args = vec!["dns", link.as_str()];
            args.extend(&servers);
            run("resolvectl", &args)?;
            // Every name goes to these servers, not only the link's own domains.
            run("resolvectl", &["domain", &link, "~."])?;
            // Encrypted when the server offers it (all the listed services do), plain otherwise.
            let _ = run("resolvectl", &["dnsovertls", &link, "opportunistic"]);
            changed += 1;
        }
        if changed > 0 {
            let _ = run("resolvectl", &["flush-caches"]);
        }
        Ok(())
    }

    pub fn restore(saved: &[Saved]) -> anyhow::Result<()> {
        for s in saved {
            // A link that is gone has nothing to put back.
            let _ = run("resolvectl", &["revert", &s.id]);
        }
        let _ = run("resolvectl", &["flush-caches"]);
        Ok(())
    }

    /// Links that carry a default route (IPv4 or IPv6).
    fn default_links() -> Vec<String> {
        let mut links = Vec::new();
        for family in ["-4", "-6"] {
            let Ok(out) = std::process::Command::new("ip")
                .args([family, "-o", "route", "show", "default"])
                .output()
            else {
                continue;
            };
            for link in route_devices(&String::from_utf8_lossy(&out.stdout)) {
                if !links.contains(&link) {
                    links.push(link);
                }
            }
        }
        links
    }

    /// The `dev` of each line of `ip -o route show default`.
    pub(super) fn route_devices(text: &str) -> Vec<String> {
        text.lines()
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                words.find(|w| *w == "dev")?;
                words.next().map(str::to_owned)
            })
            .collect()
    }

    pub(super) fn nameservers(resolv_conf: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for line in resolv_conf.lines() {
            let mut words = line.split_whitespace();
            if words.next() == Some("nameserver")
                && let Some(server) = words.next()
                && !out.iter().any(|s| s == server)
            {
                out.push(server.to_owned());
            }
        }
        out
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use dpimech_core::model::DnsInUse;

    use super::Saved;

    pub fn apply(
        _v4: &[&str],
        _v6: &[&str],
        _saved: &mut Vec<Saved>,
        _only_new: bool,
    ) -> anyhow::Result<()> {
        anyhow::bail!("setting DNS from DPIMech is not available on macOS yet")
    }

    pub fn restore(_saved: &[Saved]) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn in_use() -> Vec<DnsInUse> {
        let Ok(out) = std::process::Command::new("scutil").arg("--dns").output() else {
            return Vec::new();
        };
        let servers = first_resolver(&String::from_utf8_lossy(&out.stdout));
        if servers.is_empty() {
            return Vec::new();
        }
        vec![DnsInUse {
            connection: "system".into(),
            servers,
            manual: None,
        }]
    }

    /// The servers of "resolver #1" (the default one) in `scutil --dns`.
    pub(super) fn first_resolver(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut inside = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with("resolver #") {
                if inside {
                    break;
                }
                inside = line == "resolver #1";
            } else if inside
                && line.starts_with("nameserver[")
                && let Some((_, server)) = line.split_once(':')
            {
                out.push(server.trim().to_owned());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn resolv_conf_nameservers() {
        let text = "# generated\nnameserver 192.168.1.1\nnameserver fe80::1%wlan0\nnameserver 192.168.1.1\nsearch lan\n";
        assert_eq!(
            super::platform::nameservers(text),
            ["192.168.1.1", "fe80::1%wlan0"]
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn default_route_devices() {
        let text = "default via 192.168.1.1 dev wlp2s0 proto dhcp src 192.168.1.20 metric 600\ndefault dev wg0 scope link\n";
        assert_eq!(super::platform::route_devices(text), ["wlp2s0", "wg0"]);
    }

    #[test]
    fn only_listed_services_can_be_set() {
        let (v4, v6) = super::addresses("Cloudflare").unwrap();
        assert_eq!(v4, ["1.1.1.1", "1.0.0.1"]);
        assert!(v6.iter().all(|a| a.contains(':')));
        assert!(super::addresses("6.6.6.6").is_none());
        assert!(super::addresses("cloudflare ").is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn scutil_first_resolver() {
        let text = "DNS configuration\n\nresolver #1\n  nameserver[0] : 192.168.1.1\n  nameserver[1] : 1.1.1.1\n  flags : Request A records\n\nresolver #2\n  domain : local\n  nameserver[0] : 224.0.0.251\n";
        assert_eq!(
            super::platform::first_resolver(text),
            ["192.168.1.1", "1.1.1.1"]
        );
    }
}
