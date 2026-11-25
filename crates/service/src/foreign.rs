//! Finds DPI bypass tools that run outside DPIMech.
//!
//! Two packet engines on the same traffic (e.g. a GoodbyeDPI the user started by hand next to
//! DPIMech's winws) rewrite each other's packets and break connections in confusing ways.
//! The service does the scan because it runs privileged: an unelevated GUI cannot read the
//! image path of SYSTEM processes, so it could not tell DPIMech's own engines from others.

use std::path::Path;

use dpimech_core::ipc::ForeignTool;

/// Lower-case executable stems and the name shown to the user.
const KNOWN: &[(&str, &str)] = &[
    ("goodbyedpi", "GoodbyeDPI"),
    ("winws", "zapret (winws)"),
    ("nfqws", "zapret (nfqws)"),
    ("tpws", "zapret (tpws)"),
    ("ciadpi", "ByeDPI"),
    ("byedpi", "ByeDPI"),
    ("byedpimanager", "ByeDPI Manager"),
    ("spoofdpi", "SpoofDPI"),
    ("proxifyre", "ProxiFyre"),
    ("powertunnel", "PowerTunnel"),
    ("green-tunnel", "Green Tunnel"),
    ("dpitunnel", "DPI Tunnel"),
    ("dpimngr-service", "dpimngr (old version of DPIMech)"),
];

/// Software that is not a DPI tool but is known to break bypassing: it filters or reroutes the
/// same traffic (the list zapret users check first when nothing helps).
const INTERFERING: &[(&str, &str)] = &[
    ("adguardsvc", "AdGuard"),
    ("adguard", "AdGuard"),
    ("killernetworkservice", "Killer Network Service"),
    ("smartbytenetworkservice", "SmartByte"),
    ("smartbyteanalyticsservice", "SmartByte"),
    ("vsmon", "ZoneAlarm / Check Point"),
    ("openvpn", "OpenVPN"),
    ("wireguard", "WireGuard"),
    ("warp-svc", "Cloudflare WARP"),
    ("protonvpnservice", "Proton VPN"),
    ("windscribeservice", "Windscribe"),
    ("nordvpn-service", "NordVPN"),
    ("nordvpnd", "NordVPN"),
    ("expressvpnd", "ExpressVPN"),
    ("expressvpn.service", "ExpressVPN"),
];

/// VPN clients keep a background service running while disconnected (Cloudflare WARP does),
/// so for them the process alone says nothing: they are only named while traffic really goes
/// through a tunnel.
const VPNS: &[&str] = &[
    "OpenVPN",
    "WireGuard",
    "Cloudflare WARP",
    "Proton VPN",
    "Windscribe",
    "NordVPN",
    "ExpressVPN",
];

/// Every known tool that is running from outside `<data>/engines`.
pub fn scan(engines_dir: &Path) -> Vec<ForeignTool> {
    let ours = normalise(&engines_dir.to_string_lossy());
    let mut found: Vec<ForeignTool> = Vec::new();
    for (name, path) in processes() {
        let Some((display, interferes)) = classify(&name) else {
            continue;
        };
        if path
            .as_deref()
            .is_some_and(|p| normalise(p).starts_with(&ours))
        {
            continue;
        }
        if !found.iter().any(|f| f.name == display) {
            found.push(ForeignTool {
                name: display.to_owned(),
                path: path.unwrap_or(name),
                interferes,
            });
        }
    }
    if found.iter().any(|t| VPNS.contains(&t.name.as_str())) && !tunnel_carries_traffic() {
        found.retain(|t| !VPNS.contains(&t.name.as_str()));
    }
    found
}

/// Whether the route to the internet goes through a virtual (tunnel) interface right now,
/// i.e. a VPN is connected. A VPN on a TAP adapter looks like Ethernet and is missed: then
/// nothing is said, which is the harmless side to err on.
#[cfg(windows)]
fn tunnel_carries_traffic() -> bool {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetBestInterface, GetIfEntry2, IF_TYPE_ETHERNET_CSMACD, IF_TYPE_IEEE80211, IF_TYPE_PPP,
        MIB_IF_ROW2,
    };
    // Mobile broadband (WWAN) adapters.
    const IF_TYPE_WWANPP: u32 = 243;
    const IF_TYPE_WWANPP2: u32 = 244;
    let mut index = 0u32;
    // SAFETY: plain out-parameters; the address is 1.1.1.1 in network byte order.
    unsafe {
        if GetBestInterface(u32::from_ne_bytes([1, 1, 1, 1]), &mut index) != 0 {
            return false;
        }
        let mut row: MIB_IF_ROW2 = std::mem::zeroed();
        row.InterfaceIndex = index;
        if GetIfEntry2(&mut row) != 0 {
            return false;
        }
        !matches!(
            row.Type,
            IF_TYPE_ETHERNET_CSMACD
                | IF_TYPE_IEEE80211
                | IF_TYPE_PPP
                | IF_TYPE_WWANPP
                | IF_TYPE_WWANPP2
        )
    }
}

/// Linux: the route's interface has no hardware device behind it (tun, wg, CloudflareWARP…).
#[cfg(all(unix, not(target_os = "macos")))]
fn tunnel_carries_traffic() -> bool {
    let Ok(out) = std::process::Command::new("ip")
        .args(["-o", "route", "get", "1.1.1.1"])
        .output()
    else {
        return false;
    };
    match route_device(&String::from_utf8_lossy(&out.stdout)) {
        Some(dev) => !Path::new("/sys/class/net")
            .join(&dev)
            .join("device")
            .exists(),
        None => false,
    }
}

/// macOS VPNs use `utun` interfaces.
#[cfg(target_os = "macos")]
fn tunnel_carries_traffic() -> bool {
    std::process::Command::new("route")
        .args(["-n", "get", "1.1.1.1"])
        .output()
        .is_ok_and(|out| {
            String::from_utf8_lossy(&out.stdout).lines().any(|l| {
                l.trim()
                    .strip_prefix("interface:")
                    .is_some_and(|i| i.trim().starts_with("utun"))
            })
        })
}

/// The `dev` in `ip route get` output.
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
fn route_device(text: &str) -> Option<String> {
    let mut words = text.split_whitespace();
    words.find(|w| *w == "dev")?;
    words.next().map(str::to_owned)
}

/// Name of the tool an executable belongs to, and whether it only interferes (not a DPI tool).
/// Unix builds of DPI tools may carry the CPU in the name (`ciadpi-x86_64`).
fn classify(file_name: &str) -> Option<(&'static str, bool)> {
    let lower = file_name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    let dpi = KNOWN.iter().find_map(|(known, display)| {
        let arch_suffixed = stem
            .strip_prefix(known)
            .is_some_and(|rest| rest.starts_with('-') && *known != "dpimngr-service");
        (stem == *known || arch_suffixed).then_some((*display, false))
    });
    dpi.or_else(|| {
        INTERFERING
            .iter()
            .find(|(known, _)| stem == *known)
            .map(|(_, display)| (*display, true))
    })
}

fn normalise(path: &str) -> String {
    let p = path.replace('\\', "/");
    if cfg!(windows) {
        p.to_ascii_lowercase()
    } else {
        p
    }
}

/// `(executable file name, full path if readable)` of every process.
#[cfg(windows)]
fn processes() -> Vec<(String, Option<String>)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };

    let mut out = Vec::new();
    // SAFETY: standard Toolhelp iteration; every handle is checked and closed.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut entry);
        while ok != 0 {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            let mut path = None;
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
            if !handle.is_null() {
                let mut buf = [0u16; 1024];
                let mut size = buf.len() as u32;
                if QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_WIN32,
                    buf.as_mut_ptr(),
                    &mut size,
                ) != 0
                {
                    path = Some(String::from_utf16_lossy(&buf[..size as usize]));
                }
                CloseHandle(handle);
            }
            out.push((name, path));
            ok = Process32NextW(snap, &mut entry);
        }
        CloseHandle(snap);
    }
    out
}

#[cfg(unix)]
fn processes() -> Vec<(String, Option<String>)> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .filter_map(|e| {
            let exe = std::fs::read_link(e.path().join("exe")).ok();
            // Scripts show the interpreter as `exe`; `comm` has the name the process runs as.
            let comm = std::fs::read_to_string(e.path().join("comm"))
                .ok()
                .map(|c| c.trim().to_owned());
            let exe_name = exe
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned());
            let name = match (&exe_name, &comm) {
                (Some(n), _) if classify(n).is_some() => n.clone(),
                (_, Some(c)) => c.clone(),
                (Some(n), None) => n.clone(),
                (None, None) => return None,
            };
            let path = exe
                .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy() == name))
                .map(|p| p.display().to_string())
                .or_else(|| cmdline_path(&e.path()));
            Some((name, path))
        })
        .collect()
}

/// First argument that is a path, e.g. the script a Python stand-in runs; lets processes
/// started through an interpreter still be matched against the engines folder.
#[cfg(unix)]
fn cmdline_path(proc_dir: &Path) -> Option<String> {
    let raw = std::fs::read(proc_dir.join("cmdline")).ok()?;
    raw.split(|&b| b == 0)
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .find(|a| a.starts_with('/') && !a.starts_with("/usr/bin/") && !a.starts_with("/bin/"))
}

#[cfg(test)]
mod tests {
    use super::{INTERFERING, VPNS, classify, route_device};

    #[test]
    fn recognises_known_tools_but_not_lookalikes() {
        let dpi = |name: &'static str| Some((name, false));
        assert_eq!(classify("GoodbyeDPI.exe"), dpi("GoodbyeDPI"));
        assert_eq!(classify("winws.exe"), dpi("zapret (winws)"));
        assert_eq!(classify("ciadpi-x86_64"), dpi("ByeDPI"));
        assert_eq!(classify("nfqws"), dpi("zapret (nfqws)"));
        assert_eq!(
            classify("dpimngr-service.exe"),
            dpi("dpimngr (old version of DPIMech)")
        );
        assert_eq!(classify("dpimech-service.exe"), None);
        assert_eq!(classify("gtk-update-icon-cache"), None);
        assert_eq!(classify("winwsx.exe"), None);
        assert_eq!(classify("chrome.exe"), None);
    }

    #[test]
    fn route_device_is_read() {
        let text = "1.1.1.1 dev CloudflareWARP table 65743 src 172.16.0.2 uid 0 \\    cache";
        assert_eq!(route_device(text).as_deref(), Some("CloudflareWARP"));
        assert_eq!(
            route_device("1.1.1.1 via 192.168.1.1 dev wlan0 src 192.168.1.20"),
            Some("wlan0".into())
        );
        assert_eq!(
            route_device("RTNETLINK answers: Network is unreachable"),
            None
        );
        // Every VPN name has a process entry, so none can be named without being found.
        for vpn in VPNS {
            assert!(INTERFERING.iter().any(|(_, name)| name == vpn), "{vpn}");
        }
    }

    #[test]
    fn software_that_gets_in_the_way_is_told_apart() {
        assert_eq!(classify("AdguardSvc.exe"), Some(("AdGuard", true)));
        assert_eq!(classify("warp-svc"), Some(("Cloudflare WARP", true)));
        assert_eq!(
            classify("KillerNetworkService.exe"),
            Some(("Killer Network Service", true))
        );
        // Exact names only: no guessing from parts of a name.
        assert_eq!(classify("openvpn-gui.exe"), None);
        assert_eq!(classify("wireguard.exe"), Some(("WireGuard", true)));
    }
}
