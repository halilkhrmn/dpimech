//! Discovers applications the user may want to route: running processes and
//! Start Menu shortcuts, each with its icon.

use slint::{Rgba8Pixel, SharedPixelBuffer};

/// Built off the UI thread, so the icon stays a pixel buffer until it reaches Slint.
pub struct FoundApp {
    /// Human-friendly name (shortcut title or executable name).
    pub title: String,
    /// Executable name without extension; this is what ProxiFyre matches on.
    pub exe: String,
    pub path: String,
    pub running: bool,
    /// Has a Start Menu shortcut / .desktop entry, i.e. a "real" app rather than a helper.
    #[cfg_attr(target_os = "macos", allow(dead_code))] // no per-app routing on macOS yet
    pub listed: bool,
    /// What a profile shortcut opens: the Start Menu shortcut or .desktop entry when there is
    /// one (it survives app updates that move the .exe), otherwise the executable.
    pub launch: String,
    pub icon: Option<SharedPixelBuffer<Rgba8Pixel>>,
}

#[cfg(windows)]
pub use windows::{discover, icon_for_path, icon_for_path_sized};

#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::discover;

#[cfg(target_os = "macos")]
pub fn discover() -> Vec<FoundApp> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn icon_for_path(_path: &str) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    None
}

/// Name used for routing: the file name without `.exe`.
pub fn exe_stem(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    name.strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".EXE"))
        .unwrap_or(name)
        .to_owned()
}

#[cfg(windows)]
mod windows {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use slint::{Rgba8Pixel, SharedPixelBuffer};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, GetObjectW,
    };
    use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GetIconInfo, HICON, ICONINFO, PrivateExtractIconsW,
    };

    use super::{FoundApp, exe_stem};

    /// Engines and system helpers never make sense as routing targets.
    const HIDDEN: &[&str] = &[
        "dpimech",
        "dpimech-service",
        "ciadpi",
        "proxifyre",
        "winws",
        "goodbyedpi",
        "update",
        "uninstall",
        "unins000",
    ];

    pub fn discover() -> Vec<FoundApp> {
        // SAFETY: initialising COM on this worker thread for SHGetFileInfo; errors are harmless.
        unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };

        let windows_dir = std::env::var("SystemRoot")
            .unwrap_or_else(|_| r"C:\Windows".into())
            .to_ascii_lowercase();
        let mut by_exe: HashMap<String, FoundApp> = HashMap::new();

        for path in running_process_paths() {
            let lower = path.to_ascii_lowercase();
            if lower.starts_with(&windows_dir) || !lower.ends_with(".exe") {
                continue;
            }
            let exe = exe_stem(&path);
            by_exe.entry(exe.to_ascii_lowercase()).or_insert(FoundApp {
                title: exe.clone(),
                exe,
                launch: path.clone(),
                path,
                running: true,
                listed: false,
                icon: None,
            });
        }

        for (title, path, lnk) in start_menu_targets() {
            let exe = exe_stem(&path);
            by_exe
                .entry(exe.to_ascii_lowercase())
                .and_modify(|app| {
                    app.title = title.clone();
                    app.listed = true;
                    app.launch = lnk.clone();
                })
                .or_insert(FoundApp {
                    title,
                    exe,
                    path,
                    running: false,
                    listed: true,
                    launch: lnk,
                    icon: None,
                });
        }

        let mut apps: Vec<FoundApp> = by_exe
            .into_values()
            .filter(|a| !HIDDEN.contains(&a.exe.to_ascii_lowercase().as_str()))
            .collect();
        for app in &mut apps {
            app.icon = icon_for_path(&app.path);
        }
        // Running real apps first, then other installed apps, then background helpers.
        let rank = |a: &FoundApp| match (a.listed, a.running) {
            (true, true) => 0,
            (true, false) => 1,
            (false, _) => 2,
        };
        apps.sort_by(|a, b| {
            rank(a)
                .cmp(&rank(b))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        apps
    }

    fn running_process_paths() -> Vec<String> {
        let mut out = Vec::new();
        // SAFETY: standard Toolhelp snapshot iteration; the handle is closed below.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return out;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut entry);
            while ok != 0 {
                if let Some(path) = process_path(entry.th32ProcessID) {
                    out.push(path);
                }
                ok = Process32NextW(snap, &mut entry);
            }
            CloseHandle(snap);
        }
        out
    }

    fn process_path(pid: u32) -> Option<String> {
        // SAFETY: the handle is checked and closed; the buffer length is passed in/out.
        unsafe {
            let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok =
                QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
            CloseHandle(handle);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    /// `(shortcut title, target exe, shortcut path)` for every Start Menu shortcut pointing
    /// at an .exe.
    fn start_menu_targets() -> Vec<(String, String, String)> {
        let mut roots = Vec::new();
        for var in ["ProgramData", "APPDATA"] {
            if let Some(base) = std::env::var_os(var) {
                roots.push(PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs"));
            }
        }
        let mut out = Vec::new();
        let mut stack = roots;
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
                    && let Some(target) = shortcut_target(&path)
                {
                    let title = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    out.push((title, target, path.display().to_string()));
                }
            }
        }
        out
    }

    fn shortcut_target(lnk_path: &Path) -> Option<String> {
        let link = lnk::ShellLink::open(lnk_path, lnk::encoding::WINDOWS_1254).ok()?;
        let target = link.link_target()?;
        if !target.to_ascii_lowercase().ends_with(".exe") || !Path::new(&target).is_file() {
            return None;
        }
        // Squirrel apps (Discord, Slack, …) launch through Update.exe --processStart App.exe.
        if exe_stem(&target).eq_ignore_ascii_case("update")
            && let Some(args) = link.string_data().command_line_arguments()
            && let Some(real) = args.split("--processStart").nth(1)
        {
            let real = real.trim().trim_matches('"').split_whitespace().next()?;
            let dir = Path::new(&target).parent()?;
            return Some(dir.join(real).to_string_lossy().into_owned());
        }
        Some(target)
    }

    /// Extracts the 32×32 shell icon of a file as RGBA.
    pub fn icon_for_path(path: &str) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        // SAFETY: every GDI object created here is released before returning.
        unsafe {
            let mut info: SHFILEINFOW = std::mem::zeroed();
            let ok = SHGetFileInfoW(
                wide.as_ptr(),
                0,
                &mut info,
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if ok == 0 || info.hIcon.is_null() {
                return None;
            }
            icon_rgba(info.hIcon)
        }
    }

    /// The icon of an executable at `size` px (e.g. 256 for shortcut icons); Windows picks the
    /// closest image in the file and scales it.
    pub fn icon_for_path_sized(path: &str, size: i32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        let mut wide = [0u16; 260];
        let encoded: Vec<u16> = path.encode_utf16().collect();
        if encoded.len() >= wide.len() {
            return None;
        }
        wide[..encoded.len()].copy_from_slice(&encoded);
        let mut icon: HICON = std::ptr::null_mut();
        let mut id = 0u32;
        // SAFETY: the path buffer is MAX_PATH long and NUL-terminated as the API requires;
        // the returned icon is destroyed by icon_rgba.
        unsafe {
            let n = PrivateExtractIconsW(wide.as_ptr(), 0, size, size, &mut icon, &mut id, 1, 0);
            if n == 0 || n == u32::MAX || icon.is_null() {
                return None;
            }
            icon_rgba(icon)
        }
    }

    /// Converts an icon to RGBA and destroys it.
    unsafe fn icon_rgba(icon: HICON) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        // SAFETY: caller passes a valid icon it owns; every GDI object is released here.
        unsafe {
            let mut icon_info: ICONINFO = std::mem::zeroed();
            let result = if GetIconInfo(icon, &mut icon_info) != 0 {
                let pixels = bitmap_rgba(icon_info.hbmColor);
                DeleteObject(icon_info.hbmColor);
                DeleteObject(icon_info.hbmMask);
                pixels
            } else {
                None
            };
            DestroyIcon(icon);
            result
        }
    }

    unsafe fn bitmap_rgba(
        bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    ) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        if bitmap.is_null() {
            return None;
        }
        // SAFETY: caller passes a valid HBITMAP; the DC is deleted before returning.
        unsafe {
            let mut bm: BITMAP = std::mem::zeroed();
            if GetObjectW(
                bitmap,
                size_of::<BITMAP>() as i32,
                &mut bm as *mut _ as *mut _,
            ) == 0
            {
                return None;
            }
            let (w, h) = (bm.bmWidth as u32, bm.bmHeight as u32);
            let mut header: BITMAPINFO = std::mem::zeroed();
            header.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32), // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bgra = vec![0u8; (w * h * 4) as usize];
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            let lines = GetDIBits(
                dc,
                bitmap,
                0,
                h,
                bgra.as_mut_ptr() as *mut _,
                &mut header,
                DIB_RGB_COLORS,
            );
            DeleteDC(dc);
            if lines == 0 {
                return None;
            }
            // Old-style icons carry no alpha channel; treat them as fully opaque.
            let has_alpha = bgra.chunks_exact(4).any(|p| p[3] != 0);
            let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
            for (dst, src) in buffer.make_mut_slice().iter_mut().zip(bgra.chunks_exact(4)) {
                *dst = Rgba8Pixel {
                    r: src[2],
                    g: src[1],
                    b: src[0],
                    a: if has_alpha { src[3] } else { 255 },
                };
            }
            Some(buffer)
        }
    }
}

/// Linux: apps from .desktop entries (system, user, Flatpak, Snap) plus the user's running
/// processes. The key is the executable name the service matches processes by.
#[cfg(all(unix, not(target_os = "macos")))]
mod linux {
    use std::collections::HashMap;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    use super::FoundApp;

    /// Desktop plumbing that is never worth routing.
    const HIDDEN: &[&str] = &[
        "dpimech",
        "dpimech-service",
        "ciadpi",
        "tpws",
        "nfqws",
        "bash",
        "sh",
        "zsh",
        "fish",
        "systemd",
        "dbus-daemon",
        "dbus-broker",
        "pipewire",
        "wireplumber",
        "pulseaudio",
        "xorg",
        "xwayland",
        "gnome-shell",
        "plasmashell",
        "kwin_wayland",
        "kwin_x11",
        "gvfsd",
        "at-spi-bus-launcher",
        "at-spi2-registryd",
        "ssh-agent",
        "gpg-agent",
        "xdg-desktop-portal",
        "xdg-document-portal",
        "xdg-permission-store",
        "bwrap",
    ];

    pub fn discover() -> Vec<FoundApp> {
        let mut by_exe: HashMap<String, FoundApp> = HashMap::new();
        for (title, exe, path) in desktop_entries() {
            by_exe.entry(exe.to_ascii_lowercase()).or_insert(FoundApp {
                title,
                exe,
                launch: path.clone(),
                path,
                running: false,
                listed: true,
                icon: None,
            });
        }
        for (exe, path) in running_processes() {
            let key = exe.to_ascii_lowercase();
            if HIDDEN.contains(&key.as_str()) {
                continue;
            }
            by_exe
                .entry(key)
                .and_modify(|a| a.running = true)
                .or_insert(FoundApp {
                    title: exe.clone(),
                    exe,
                    launch: path.clone(),
                    path,
                    running: true,
                    listed: false,
                    icon: None,
                });
        }
        let mut apps: Vec<FoundApp> = by_exe.into_values().collect();
        // Running real apps first, then other real apps, then helper processes.
        apps.sort_by_key(|a| (!(a.listed && a.running), !a.listed, a.title.to_lowercase()));
        apps
    }

    fn application_dirs() -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = [
            "/usr/share/applications",
            "/usr/local/share/applications",
            "/var/lib/flatpak/exports/share/applications",
            "/var/lib/snapd/desktop/applications",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            dirs.push(home.join(".local/share/applications"));
            dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
        }
        dirs
    }

    /// `(title, executable name, .desktop path)` of every visible application.
    fn desktop_entries() -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for dir in application_dirs() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "desktop")
                    && let Ok(text) = std::fs::read_to_string(&path)
                    && let Some((title, exe)) = parse_desktop(&text)
                {
                    out.push((title, exe, path.display().to_string()));
                }
            }
        }
        out
    }

    /// Name and executable of an application entry; `None` for hidden or non-app entries.
    pub(super) fn parse_desktop(text: &str) -> Option<(String, String)> {
        let mut in_entry = false;
        let (mut name, mut exec, mut wm_class) = (None, None, None);
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                in_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_entry {
                continue;
            }
            match line.split_once('=') {
                Some(("Type", v)) if v != "Application" => return None,
                Some(("NoDisplay" | "Hidden", "true")) => return None,
                Some(("Name", v)) => name = Some(v.to_owned()),
                Some(("Exec", v)) => exec = Some(v.to_owned()),
                Some(("StartupWMClass", v)) => wm_class = Some(v.to_owned()),
                _ => {}
            }
        }
        let exe = executable(&exec?, wm_class.as_deref())?;
        Some((name.unwrap_or_else(|| exe.clone()), exe))
    }

    /// The process name an Exec line ends up running: the program's file name, the app id's
    /// last part for Flatpak (`com.discordapp.Discord` → `Discord`), skipping `env VAR=…`.
    fn executable(exec: &str, wm_class: Option<&str>) -> Option<String> {
        let tokens: Vec<&str> = exec
            .split_whitespace()
            .map(|t| t.trim_matches('"'))
            .filter(|t| !t.starts_with('%'))
            .collect();
        let mut rest = tokens
            .iter()
            .skip_while(|t| **t == "env" || t.contains('='));
        let program = *rest.next()?;
        let base = program.rsplit('/').next().unwrap_or(program);
        if base == "flatpak" {
            let app_id = rest
                .skip_while(|t| **t != "run")
                .skip(1)
                .find(|t| !t.starts_with('-'))?;
            return Some(
                wm_class
                    .map(str::to_owned)
                    .unwrap_or_else(|| app_id.rsplit('.').next().unwrap_or(app_id).to_owned()),
            );
        }
        (!base.is_empty()).then(|| base.to_owned())
    }

    /// `(executable name, path)` of this user's processes.
    fn running_processes() -> Vec<(String, String)> {
        // SAFETY: getuid has no preconditions.
        let uid = unsafe { libc_getuid() };
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
            .filter(|e| e.metadata().is_ok_and(|m| m.uid() == uid))
            .filter_map(|e| std::fs::read_link(e.path().join("exe")).ok())
            .filter(|exe| !exe.starts_with("/usr/libexec") && !exe.starts_with("/usr/lib/systemd"))
            .filter_map(|exe| {
                let name = Path::new(&exe).file_name()?.to_string_lossy().into_owned();
                Some((name, exe.display().to_string()))
            })
            .collect()
    }

    unsafe extern "C" {
        #[link_name = "getuid"]
        fn libc_getuid() -> u32;
    }

    #[cfg(test)]
    mod tests {
        use super::parse_desktop;

        #[test]
        fn desktop_entries_give_the_process_name() {
            let native = "[Desktop Entry]\nType=Application\nName=Discord\nExec=/usr/share/discord/Discord %U\n";
            assert_eq!(
                parse_desktop(native),
                Some(("Discord".into(), "Discord".into()))
            );
            let flatpak = "[Desktop Entry]\nName=Discord\nExec=/usr/bin/flatpak run --branch=stable --command=com.discordapp.Discord com.discordapp.Discord\n";
            assert_eq!(parse_desktop(flatpak).unwrap().1, "Discord");
            let env = "[Desktop Entry]\nName=Telegram\nExec=env QT_QPA_PLATFORM=xcb telegram-desktop -- %u\n";
            assert_eq!(parse_desktop(env).unwrap().1, "telegram-desktop");
            assert_eq!(
                parse_desktop("[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n"),
                None
            );
            let actions = "[Desktop Entry]\nName=Firefox\nExec=firefox %u\n[Desktop Action new]\nExec=other\n";
            assert_eq!(parse_desktop(actions).unwrap().1, "firefox");
        }
    }
}
