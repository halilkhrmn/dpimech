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

/// Every known tool that is running from outside `<data>/engines`.
pub fn scan(engines_dir: &Path) -> Vec<ForeignTool> {
    let ours = normalise(&engines_dir.to_string_lossy());
    let mut found: Vec<ForeignTool> = Vec::new();
    for (name, path) in processes() {
        let Some(display) = classify(&name) else {
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
            });
        }
    }
    found
}

/// Name of the tool an executable belongs to. Unix builds may carry the CPU in the name
/// (`ciadpi-x86_64`).
fn classify(file_name: &str) -> Option<&'static str> {
    let lower = file_name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    KNOWN.iter().find_map(|(known, display)| {
        let arch_suffixed = stem
            .strip_prefix(known)
            .is_some_and(|rest| rest.starts_with('-') && *known != "dpimngr-service");
        (stem == *known || arch_suffixed).then_some(*display)
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
    use super::classify;

    #[test]
    fn recognises_known_tools_but_not_lookalikes() {
        assert_eq!(classify("GoodbyeDPI.exe"), Some("GoodbyeDPI"));
        assert_eq!(classify("winws.exe"), Some("zapret (winws)"));
        assert_eq!(classify("ciadpi-x86_64"), Some("ByeDPI"));
        assert_eq!(classify("nfqws"), Some("zapret (nfqws)"));
        assert_eq!(
            classify("dpimngr-service.exe"),
            Some("dpimngr (old version of DPIMech)")
        );
        assert_eq!(classify("dpimech-service.exe"), None);
        assert_eq!(classify("gtk-update-icon-cache"), None);
        assert_eq!(classify("winwsx.exe"), None);
        assert_eq!(classify("chrome.exe"), None);
    }
}
