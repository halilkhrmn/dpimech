//! Profile shortcuts: a desktop / menu entry that starts one profile and then opens an app.
//! The entry runs `dpimech --launch <profile> [--open <target>]` (see `launcher.rs`); its icon
//! is the DPIMech logo with the app's icon in the top-right corner.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use dpimech_core::model::{Profile, Routing};
use slint::{Rgba8Pixel, SharedPixelBuffer};

use crate::apps;

pub type Rgba = SharedPixelBuffer<Rgba8Pixel>;

pub const LAUNCH_FLAG: &str = "--launch";
pub const OPEN_FLAG: &str = "--open";

/// An app the shortcut can open after the profile is on.
#[derive(Clone)]
pub struct Choice {
    pub title: String,
    /// Passed to `--open`: a Start Menu shortcut, .desktop entry or executable.
    pub launch: String,
    /// Where the app's icon comes from (the executable on Windows, the .desktop entry on Linux).
    #[cfg_attr(target_os = "macos", allow(dead_code))] // no app icons on macOS yet
    pub icon_source: String,
}

pub struct Request {
    pub profile_id: String,
    pub name: String,
    pub open: Option<Choice>,
    pub desktop: bool,
    pub menu: bool,
}

/// Apps offered for a profile: its own apps for per-app profiles (the ones that are
/// installed), otherwise every installed app. Slow (scans the system): call off the UI thread.
pub fn choices(profile: &Profile) -> Vec<Choice> {
    let found = apps::discover();
    let to_choice = |a: &apps::FoundApp| Choice {
        title: a.title.clone(),
        launch: a.launch.clone(),
        icon_source: if cfg!(windows) {
            a.path.clone()
        } else {
            a.launch.clone()
        },
    };
    match &profile.routing {
        Routing::PerApp { apps: keys, .. } => keys
            .iter()
            .filter_map(|key| found.iter().find(|a| a.exe.eq_ignore_ascii_case(key)))
            .map(to_choice)
            .collect(),
        _ => found.iter().filter(|a| a.listed).map(to_choice).collect(),
    }
}

/// The DPIMech logo at `size` px with the app's icon on a small plate in the top-right corner.
pub fn icon(app: Option<&Rgba>, size: u32) -> Rgba {
    let mut out = resize(&logo(), size, size);
    let Some(app) = app else {
        return out;
    };
    let plate = size * 11 / 20;
    let inner = plate * 5 / 6;
    let (px, py) = (size - plate, 0);
    let radius = plate as f32 * 0.22;
    let pixels = out.make_mut_slice();
    // A light rounded plate keeps dark app icons visible on the purple logo.
    for y in 0..plate {
        for x in 0..plate {
            let coverage =
                rounded_rect_coverage(x as f32 + 0.5, y as f32 + 0.5, plate as f32, radius);
            if coverage > 0.0 {
                let border = rounded_rect_coverage(
                    x as f32 + 0.5 - 1.0,
                    y as f32 + 0.5 - 1.0,
                    plate as f32 - 2.0,
                    radius - 1.0,
                );
                let shade = if border > 0.5 { 255 } else { 200 };
                let dst = &mut pixels[((py + y) * size + px + x) as usize];
                blend(
                    dst,
                    Rgba8Pixel {
                        r: shade,
                        g: shade,
                        b: shade,
                        a: (coverage * 255.0) as u8,
                    },
                );
            }
        }
    }
    let app = resize(app, inner, inner);
    let offset = (plate - inner) / 2;
    for y in 0..inner {
        for x in 0..inner {
            let src = app.as_slice()[(y * inner + x) as usize];
            let dst = &mut pixels[((py + offset + y) * size + px + offset + x) as usize];
            blend(dst, src);
        }
    }
    out
}

/// The app's icon as large as the system has it.
pub fn app_icon(choice: &Choice) -> Option<Rgba> {
    #[cfg(windows)]
    {
        apps::icon_for_path_sized(&choice.icon_source, 256)
            .or_else(|| apps::icon_for_path(&choice.icon_source))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        linux::desktop_icon(Path::new(&choice.icon_source))
    }
    #[cfg(target_os = "macos")]
    {
        let _ = choice;
        None
    }
}

/// Creates the shortcut(s) and returns where they were put. Shortcuts made earlier for the
/// same profile are replaced, even if their name or place was different.
pub fn create(req: &Request) -> anyhow::Result<Vec<PathBuf>> {
    if !req.desktop && !req.menu {
        bail!("{}", crate::i18n::tr("Choose where to put the shortcut."));
    }
    let name = file_name(&req.name);
    if name.is_empty() {
        bail!("{}", crate::i18n::tr("Give the shortcut a name."));
    }
    let app_icon = req.open.as_ref().and_then(app_icon);
    let icon = icon(app_icon.as_ref(), 256);
    let mut args = vec![LAUNCH_FLAG.to_owned(), req.profile_id.clone()];
    if let Some(open) = &req.open {
        args.push(OPEN_FLAG.to_owned());
        args.push(open.launch.clone());
    }
    let made = platform::create(req, &name, &args, &icon)?;
    let mut registry = Registry::load();
    let mut old = registry.take(&req.profile_id);
    old.extend(unrecorded(&req.profile_id));
    for old in old {
        if !made.iter().any(|m| same_path(m, &old)) {
            remove_shortcut(&old);
        }
    }
    remove_icons(&req.profile_id, true);
    registry.entries.extend(made.iter().map(|path| Entry {
        profile: req.profile_id.clone(),
        path: path.clone(),
    }));
    registry.save();
    Ok(made)
}

/// Deletes every shortcut made for a profile (when the profile is deleted); returns how many.
pub fn remove_for(profile_id: &str) -> usize {
    let mut registry = Registry::load();
    let mut paths = registry.take(profile_id);
    paths.extend(unrecorded(profile_id));
    // Menu entries have a fixed name, so they are found even without the list.
    if let Some(menu) = platform::menu_entry(profile_id)
        && !paths.contains(&menu)
    {
        paths.push(menu);
    }
    registry.save();
    remove_icons(profile_id, false);
    paths.iter().filter(|p| remove_shortcut(p)).count()
}

/// Deletes every shortcut DPIMech made (Settings); returns how many.
pub fn remove_all() -> usize {
    let mut profiles: Vec<String> = existing().into_iter().map(|e| e.profile).collect();
    profiles.sort();
    profiles.dedup();
    profiles.iter().map(|p| remove_for(p)).sum()
}

/// Shortcuts that still exist.
pub fn count() -> usize {
    existing().len()
}

/// Recorded shortcuts that still exist, plus ours found in the usual folders without a record:
/// made before 0.2.2 (no list yet), or the list was lost.
fn existing() -> Vec<Entry> {
    let mut out: Vec<Entry> = Registry::load()
        .entries
        .into_iter()
        .filter(|e| e.path.exists())
        .collect();
    for found in scan() {
        if !out.iter().any(|e| same_path(&e.path, &found.path)) {
            out.push(found);
        }
    }
    out
}

/// Our shortcuts for `profile_id` that are not in the list.
fn unrecorded(profile_id: &str) -> Vec<PathBuf> {
    let recorded = Registry::load().entries;
    scan()
        .into_iter()
        .filter(|f| f.profile == profile_id)
        .filter(|f| !recorded.iter().any(|e| same_path(&e.path, &f.path)))
        .map(|f| f.path)
        .collect()
}

/// Shortcuts in the desktop and menu folders whose command is `dpimech --launch <profile>`.
fn scan() -> Vec<Entry> {
    platform::shortcut_dirs()
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten().map(|e| e.path()))
        .filter_map(|path| {
            let profile = launch_profile(&path)?;
            Some(Entry { profile, path })
        })
        .collect()
}

/// Windows paths differ only in case when they are the same file.
fn same_path(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// Where each shortcut went, so it can be replaced or removed later (`shortcuts.toml`).
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Registry {
    #[serde(default, rename = "shortcut")]
    entries: Vec<Entry>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Entry {
    profile: String,
    path: PathBuf,
}

impl Registry {
    fn load() -> Registry {
        crate::prefs::config_file("shortcuts.toml")
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        let Some(path) = crate::prefs::config_file("shortcuts.toml") else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }

    /// Removes and returns a profile's entries.
    fn take(&mut self, profile: &str) -> Vec<PathBuf> {
        let (mine, rest) = std::mem::take(&mut self.entries)
            .into_iter()
            .partition(|e| e.profile == profile);
        self.entries = rest;
        mine.into_iter().map(|e: Entry| e.path).collect()
    }
}

/// Deletes a shortcut, but only one that is really ours: its command is `dpimech --launch`.
/// The list is a user-writable file, so it must not be able to point us at anything else.
fn remove_shortcut(path: &Path) -> bool {
    if !is_our_shortcut(path) {
        return false;
    }
    let removed = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    removed.is_ok()
}

fn is_our_shortcut(path: &Path) -> bool {
    launch_profile(path).is_some()
}

/// The profile a shortcut starts, if its command is `dpimech --launch <profile>`.
fn launch_profile(path: &Path) -> Option<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let command = match ext.as_str() {
        // .lnk files store the arguments as UTF-16.
        "lnk" => {
            let bytes = std::fs::read(path).ok()?;
            let wide: Vec<u8> = LAUNCH_FLAG
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();
            let at = bytes
                .windows(wide.len())
                .position(|w| w == wide.as_slice())?;
            let units: Vec<u16> = bytes[at..]
                .chunks_exact(2)
                .take(200)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        }
        "desktop" => std::fs::read_to_string(path).ok()?,
        "app" => std::fs::read_to_string(path.join("Contents/MacOS/launch")).ok()?,
        _ => return None,
    };
    profile_after_flag(&command)
}

fn profile_after_flag(command: &str) -> Option<String> {
    let rest = &command[command.find(LAUNCH_FLAG)? + LAUNCH_FLAG.len()..];
    let rest = rest.trim_start_matches(|c: char| c == '"' || c == '\'' || c.is_whitespace());
    let id: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (!id.is_empty()).then_some(id)
}

/// Icon files are named `<profile>-<timestamp>.<ext>`; `keep_newest` keeps the one just made.
fn remove_icons(profile_id: &str, keep_newest: bool) {
    let Some(dir) = platform::icon_dir() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let prefix = format!("{}-", safe_id(profile_id));
    let mut icons: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix(&prefix))
                .is_some_and(|rest| {
                    rest.split('.')
                        .next()
                        .is_some_and(|t| t.bytes().all(|b| b.is_ascii_digit()))
                })
        })
        .collect();
    icons.sort();
    if keep_newest {
        icons.pop();
    }
    for icon in icons {
        let _ = std::fs::remove_file(icon);
    }
}

fn safe_id(profile_id: &str) -> String {
    profile_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// The program a shortcut should start: the AppImage itself rather than its temporary mount.
pub fn launcher_exe() -> anyhow::Result<PathBuf> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(appimage));
    }
    std::env::current_exe().context("finding the DPIMech executable")
}

/// Shortcut name usable as a file name on every OS.
fn file_name(name: &str) -> String {
    name.chars()
        .filter(|c| {
            !matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') && !c.is_control()
        })
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .to_owned()
}

/// Icons are written with a timestamp in the name: Explorer and file managers cache icons by
/// path, so a recreated shortcut would otherwise keep showing the old picture.
#[cfg_attr(target_os = "macos", allow(dead_code))] // the .icns lives inside the bundle
fn icon_path(dir: &Path, profile_id: &str, ext: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    dir.join(format!("{}-{stamp}.{ext}", safe_id(profile_id)))
}

fn logo() -> Rgba {
    decode_png(include_bytes!("../assets/dpimech.png")).expect("bundled logo is a valid PNG")
}

fn decode_png(bytes: &[u8]) -> Option<Rgba> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let data = &buf[..info.buffer_size()];
    let mut out = Rgba::new(info.width, info.height);
    let pixels = out.make_mut_slice();
    match info.color_type {
        png::ColorType::Rgba => {
            for (dst, s) in pixels.iter_mut().zip(data.chunks_exact(4)) {
                *dst = Rgba8Pixel {
                    r: s[0],
                    g: s[1],
                    b: s[2],
                    a: s[3],
                };
            }
        }
        png::ColorType::Rgb => {
            for (dst, s) in pixels.iter_mut().zip(data.chunks_exact(3)) {
                *dst = Rgba8Pixel {
                    r: s[0],
                    g: s[1],
                    b: s[2],
                    a: 255,
                };
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for (dst, s) in pixels.iter_mut().zip(data.chunks_exact(2)) {
                *dst = Rgba8Pixel {
                    r: s[0],
                    g: s[0],
                    b: s[0],
                    a: s[1],
                };
            }
        }
        png::ColorType::Grayscale => {
            for (dst, s) in pixels.iter_mut().zip(data) {
                *dst = Rgba8Pixel {
                    r: *s,
                    g: *s,
                    b: *s,
                    a: 255,
                };
            }
        }
        png::ColorType::Indexed => return None, // expanded by normalize_to_color8
    }
    Some(out)
}

pub fn encode_png(image: &Rgba) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, image.width(), image.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let data: Vec<u8> = image
        .as_slice()
        .iter()
        .flat_map(|p| [p.r, p.g, p.b, p.a])
        .collect();
    if let Ok(mut writer) = encoder.write_header() {
        let _ = writer.write_image_data(&data);
    }
    out
}

/// Area-averaging downscale / bilinear upscale in premultiplied alpha, so edges of
/// transparent icons do not get dark fringes.
pub fn resize(src: &Rgba, w: u32, h: u32) -> Rgba {
    let (sw, sh) = (src.width().max(1), src.height().max(1));
    let mut out = Rgba::new(w, h);
    if w == 0 || h == 0 {
        return out;
    }
    let s = src.as_slice();
    let premul = |p: Rgba8Pixel| {
        let a = p.a as f32 / 255.0;
        [p.r as f32 * a, p.g as f32 * a, p.b as f32 * a, p.a as f32]
    };
    let (fx, fy) = (sw as f32 / w as f32, sh as f32 / h as f32);
    for (i, dst) in out.make_mut_slice().iter_mut().enumerate() {
        let (x, y) = ((i as u32 % w) as f32, (i as u32 / w) as f32);
        let mut acc = [0f32; 4];
        if fx >= 1.0 && fy >= 1.0 {
            let (x0, x1) = ((x * fx) as u32, (((x + 1.0) * fx).ceil() as u32).min(sw));
            let (y0, y1) = ((y * fy) as u32, (((y + 1.0) * fy).ceil() as u32).min(sh));
            let mut n = 0.0;
            for yy in y0..y1.max(y0 + 1) {
                for xx in x0..x1.max(x0 + 1) {
                    let p = premul(s[(yy.min(sh - 1) * sw + xx.min(sw - 1)) as usize]);
                    for c in 0..4 {
                        acc[c] += p[c];
                    }
                    n += 1.0;
                }
            }
            acc.iter_mut().for_each(|v| *v /= n);
        } else {
            let sx = ((x + 0.5) * fx - 0.5).clamp(0.0, (sw - 1) as f32);
            let sy = ((y + 0.5) * fy - 0.5).clamp(0.0, (sh - 1) as f32);
            let (x0, y0) = (sx.floor() as u32, sy.floor() as u32);
            let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
            let (tx, ty) = (sx - x0 as f32, sy - y0 as f32);
            let at = |xx: u32, yy: u32| premul(s[(yy * sw + xx) as usize]);
            let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
            for k in 0..4 {
                let top = a[k] + (b[k] - a[k]) * tx;
                let bottom = c[k] + (d[k] - c[k]) * tx;
                acc[k] = top + (bottom - top) * ty;
            }
        }
        let alpha = acc[3];
        let un = |v: f32| {
            if alpha > 0.0 {
                (v * 255.0 / alpha).round().clamp(0.0, 255.0) as u8
            } else {
                0
            }
        };
        *dst = Rgba8Pixel {
            r: un(acc[0]),
            g: un(acc[1]),
            b: un(acc[2]),
            a: alpha.round().clamp(0.0, 255.0) as u8,
        };
    }
    out
}

/// "Source over" blending of straight-alpha pixels.
fn blend(dst: &mut Rgba8Pixel, src: Rgba8Pixel) {
    let sa = src.a as f32 / 255.0;
    let da = dst.a as f32 / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.0 {
        *dst = Rgba8Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        return;
    }
    let mix = |s: u8, d: u8| {
        ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / oa)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    *dst = Rgba8Pixel {
        r: mix(src.r, dst.r),
        g: mix(src.g, dst.g),
        b: mix(src.b, dst.b),
        a: (oa * 255.0).round() as u8,
    };
}

/// How much of the pixel centred at (x, y) lies inside a `size`-square with rounded corners.
fn rounded_rect_coverage(x: f32, y: f32, size: f32, radius: f32) -> f32 {
    if x < 0.0 || y < 0.0 || x > size || y > size {
        return 0.0;
    }
    let r = radius.max(0.0);
    let cx = x.clamp(r, size - r);
    let cy = y.clamp(r, size - r);
    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    (r - d + 0.5).clamp(0.0, 1.0)
}

/// .ico with PNG-compressed images (supported since Windows Vista).
#[cfg_attr(not(windows), allow(dead_code))]
fn encode_ico(icon: &Rgba) -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = [256, 64, 48, 32, 16]
        .iter()
        .map(|&s| (s, encode_png(&resize(icon, s, s))))
        .collect();
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in &images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend_from_slice(&png);
    }
    out
}

/// .icns with PNG payloads: ic08 = 256 px, ic07 = 128 px.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn encode_icns(icon: &Rgba) -> Vec<u8> {
    let mut body = Vec::new();
    for (kind, size) in [(b"ic08", 256), (b"ic07", 128)] {
        let png = encode_png(&resize(icon, size, size));
        body.extend_from_slice(kind);
        body.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut out = b"icns".to_vec();
    out.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    out.extend_from_slice(&body);
    out
}

#[cfg(windows)]
mod platform {
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;

    use anyhow::{Context, bail};

    use super::{Request, Rgba, encode_ico, icon_path, launcher_exe};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn icon_dir() -> Option<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA")?;
        Some(PathBuf::from(base).join("DPIMech").join("shortcuts"))
    }

    /// Windows shortcuts are named by the user; only the list knows them.
    pub fn menu_entry(_profile_id: &str) -> Option<PathBuf> {
        None
    }

    /// Where shortcuts are made: the desktop and the Start menu (known folders, so a desktop
    /// moved to OneDrive is found too).
    pub fn shortcut_dirs() -> Vec<PathBuf> {
        use windows_sys::Win32::System::Com::CoTaskMemFree;
        use windows_sys::Win32::UI::Shell::{
            FOLDERID_Desktop, FOLDERID_Programs, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
        };
        [FOLDERID_Desktop, FOLDERID_Programs]
            .iter()
            .filter_map(|id| {
                let mut raw = std::ptr::null_mut();
                // SAFETY: `raw` receives a CoTaskMem string that is freed below in every case.
                let hr = unsafe {
                    SHGetKnownFolderPath(id, KF_FLAG_DEFAULT as u32, std::ptr::null_mut(), &mut raw)
                };
                let path = (hr == 0 && !raw.is_null()).then(|| {
                    // SAFETY: on success `raw` is a NUL-terminated wide string.
                    let len = (0..).take_while(|&i| unsafe { *raw.add(i) } != 0).count();
                    let wide = unsafe { std::slice::from_raw_parts(raw, len) };
                    PathBuf::from(String::from_utf16_lossy(wide))
                });
                unsafe { CoTaskMemFree(raw.cast()) };
                path
            })
            .collect()
    }

    /// WScript.Shell through PowerShell: writing .lnk files by hand means implementing
    /// MS-SHLLINK, and windows-sys has no IShellLink bindings. Values travel in environment
    /// variables, never inside the script text.
    const SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$dir = [Environment]::GetFolderPath($env:DPIMECH_FOLDER)
$path = Join-Path $dir $env:DPIMECH_FILE
$s = (New-Object -ComObject WScript.Shell).CreateShortcut($path)
$s.TargetPath = $env:DPIMECH_TARGET
$s.Arguments = $env:DPIMECH_ARGS
$s.WorkingDirectory = Split-Path -Parent $env:DPIMECH_TARGET
$s.IconLocation = $env:DPIMECH_ICON + ',0'
$s.Description = $env:DPIMECH_DESC
$s.Save()
[Console]::Out.Write($path)"#;

    pub fn create(
        req: &Request,
        name: &str,
        args: &[String],
        icon: &Rgba,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let dir = icon_dir().context("LOCALAPPDATA is not set")?;
        std::fs::create_dir_all(&dir)?;
        let ico = icon_path(&dir, &req.profile_id, "ico");
        std::fs::write(&ico, encode_ico(icon)).context("writing the shortcut icon")?;

        let exe = launcher_exe()?;
        // Windows paths cannot contain quotes, so quoting each argument is enough.
        let arguments = args
            .iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(" ");
        let description = trf!("Starts the DPIMech profile \"{}\"", req.name);
        let mut made = Vec::new();
        for (wanted, folder) in [(req.desktop, "Desktop"), (req.menu, "Programs")] {
            if !wanted {
                continue;
            }
            let out = Command::new("powershell.exe")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    SCRIPT,
                ])
                .env("DPIMECH_FOLDER", folder)
                .env("DPIMECH_FILE", format!("{name}.lnk"))
                .env("DPIMECH_TARGET", &exe)
                .env("DPIMECH_ARGS", &arguments)
                .env("DPIMECH_ICON", &ico)
                .env("DPIMECH_DESC", &description)
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .context("running PowerShell")?;
            if !out.status.success() {
                bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
            }
            made.push(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()));
        }
        Ok(made)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use anyhow::Context;

    use super::{Request, Rgba, encode_png, icon_path, launcher_exe};

    pub fn create(
        req: &Request,
        name: &str,
        args: &[String],
        icon: &Rgba,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let dir = icon_dir().context("HOME is not set")?;
        std::fs::create_dir_all(&dir)?;
        let png = icon_path(&dir, &req.profile_id, "png");
        std::fs::write(&png, encode_png(icon)).context("writing the shortcut icon")?;

        let exe = launcher_exe()?;
        let exec = std::iter::once(exe.display().to_string())
            .chain(args.iter().cloned())
            .map(|a| desktop_quote(&a))
            .collect::<Vec<_>>()
            .join(" ");
        let comment = trf!("Starts the DPIMech profile \"{}\"", req.name);
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={exec}\nIcon={}\nTerminal=false\nCategories=Network;\nStartupNotify=false\n",
            one_line(&req.name),
            one_line(&comment),
            png.display()
        );

        let mut targets = Vec::new();
        if req.menu {
            targets.extend(menu_entry(&req.profile_id));
        }
        if req.desktop {
            targets.push(super::linux::desktop_dir().join(format!("{name}.desktop")));
        }
        for path in &targets {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, &entry).with_context(|| format!("writing {}", path.display()))?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
            // GNOME only runs desktop launchers marked as trusted; other desktops ignore this.
            let _ = std::process::Command::new("gio")
                .args([
                    "set",
                    &path.display().to_string(),
                    "metadata::trusted",
                    "true",
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        Ok(targets)
    }

    pub fn icon_dir() -> Option<PathBuf> {
        Some(super::linux::data_home()?.join("dpimech").join("shortcuts"))
    }

    pub fn shortcut_dirs() -> Vec<PathBuf> {
        let mut dirs = vec![super::linux::desktop_dir()];
        dirs.extend(super::linux::data_home().map(|d| d.join("applications")));
        dirs
    }

    pub fn menu_entry(profile_id: &str) -> Option<PathBuf> {
        Some(
            super::linux::data_home()?
                .join("applications")
                .join(format!(
                    "dpimech-profile-{}.desktop",
                    super::safe_id(profile_id)
                )),
        )
    }

    fn one_line(s: &str) -> String {
        s.replace(['\n', '\r'], " ")
    }

    /// Desktop Entry spec: quote the argument, escape `"` `` ` `` `$` `\` inside the quotes,
    /// then escape backslashes again for the string value and double `%` (field codes).
    pub(super) fn desktop_quote(arg: &str) -> String {
        let mut quoted = String::from("\"");
        for c in arg.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        quoted.replace('\\', "\\\\").replace('%', "%%")
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use anyhow::Context;

    use super::{Request, Rgba, encode_icns, launcher_exe};

    /// A tiny .app bundle whose executable is a shell script: Finder shows it with our icon
    /// and it can live on the Desktop, in Applications or in the Dock.
    pub fn create(
        req: &Request,
        name: &str,
        args: &[String],
        icon: &Rgba,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
        let exe = launcher_exe()?;
        let script = format!(
            "#!/bin/sh\nexec {}\n",
            std::iter::once(exe.display().to_string())
                .chain(args.iter().cloned())
                .map(|a| sh_quote(&a))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let id: String = req
            .profile_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>{}</string>
    <key>CFBundleIdentifier</key><string>io.github.dpimech.shortcut.{id}</string>
    <key>CFBundleExecutable</key><string>launch</string>
    <key>CFBundleIconFile</key><string>icon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSUIElement</key><true/>
</dict>
</plist>
"#,
            xml_escape(&req.name)
        );
        let mut made = Vec::new();
        for (wanted, dir) in [(req.desktop, "Desktop"), (req.menu, "Applications")] {
            if !wanted {
                continue;
            }
            let bundle = home.join(dir).join(format!("{name}.app"));
            let contents = bundle.join("Contents");
            std::fs::create_dir_all(contents.join("MacOS"))?;
            std::fs::create_dir_all(contents.join("Resources"))?;
            std::fs::write(contents.join("Info.plist"), &plist)?;
            std::fs::write(
                contents.join("Resources").join("icon.icns"),
                encode_icns(icon),
            )?;
            let launch = contents.join("MacOS").join("launch");
            std::fs::write(&launch, &script)?;
            std::fs::set_permissions(&launch, std::fs::Permissions::from_mode(0o755))?;
            made.push(bundle);
        }
        Ok(made)
    }

    /// The icon lives inside each .app bundle.
    pub fn icon_dir() -> Option<PathBuf> {
        None
    }

    pub fn menu_entry(_profile_id: &str) -> Option<PathBuf> {
        None
    }

    pub fn shortcut_dirs() -> Vec<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| vec![home.join("Desktop"), home.join("Applications")])
            .unwrap_or_default()
    }

    fn sh_quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', r"'\''"))
    }

    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

/// Linux helpers shared by the shortcut writer and the launcher.
#[cfg(all(unix, not(target_os = "macos")))]
pub mod linux {
    use std::path::{Path, PathBuf};

    use super::Rgba;

    pub fn data_home() -> Option<PathBuf> {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    }

    /// The desktop folder from `user-dirs.dirs` (localised, e.g. `~/Masaüstü`).
    pub fn desktop_dir() -> PathBuf {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        std::fs::read_to_string(config.join("user-dirs.dirs"))
            .ok()
            .and_then(|text| parse_user_dirs(&text, &home))
            .unwrap_or_else(|| home.join("Desktop"))
    }

    pub(super) fn parse_user_dirs(text: &str, home: &Path) -> Option<PathBuf> {
        let line = text
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with("XDG_DESKTOP_DIR="))?;
        let value = line.split_once('=')?.1.trim().trim_matches('"');
        let path = match value.strip_prefix("$HOME") {
            Some(rest) => home.join(rest.trim_start_matches('/')),
            None => PathBuf::from(value),
        };
        (path.is_absolute() && path != home).then_some(path)
    }

    /// A key from the `[Desktop Entry]` group.
    pub fn desktop_value(text: &str, key: &str) -> Option<String> {
        let mut in_entry = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                in_entry = line == "[Desktop Entry]";
            } else if in_entry
                && let Some((k, v)) = line.split_once('=')
                && k.trim() == key
            {
                return Some(v.trim().to_owned());
            }
        }
        None
    }

    /// The icon named in a .desktop entry, looked up like a desktop would (largest first).
    pub fn desktop_icon(desktop: &Path) -> Option<Rgba> {
        let text = std::fs::read_to_string(desktop).ok()?;
        let name = desktop_value(&text, "Icon")?;
        let path = if Path::new(&name).is_absolute() {
            PathBuf::from(&name)
        } else {
            find_icon(&name)?
        };
        // Slint decodes PNG, JPEG and SVG (SVGs render at their nominal size).
        slint::Image::load_from_path(&path).ok()?.to_rgba8()
    }

    fn find_icon(name: &str) -> Option<PathBuf> {
        let mut bases: Vec<PathBuf> = Vec::new();
        if let Some(data) = data_home() {
            bases.push(data.join("icons"));
            bases.push(data.join("flatpak/exports/share/icons"));
        }
        for base in [
            "/usr/share/icons",
            "/usr/local/share/icons",
            "/var/lib/flatpak/exports/share/icons",
            "/var/lib/snapd/desktop/icons",
        ] {
            bases.push(PathBuf::from(base));
        }
        let sizes = [
            "512x512", "256x256", "192x192", "128x128", "96x96", "64x64", "48x48", "scalable",
        ];
        for base in &bases {
            for size in sizes {
                for ext in ["png", "svg"] {
                    let p = base
                        .join("hicolor")
                        .join(size)
                        .join("apps")
                        .join(format!("{name}.{ext}"));
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
        for base in &bases {
            if let Ok(themes) = std::fs::read_dir(base) {
                for theme in themes.flatten() {
                    for size in sizes {
                        for ext in ["png", "svg"] {
                            let p = theme
                                .path()
                                .join(size)
                                .join("apps")
                                .join(format!("{name}.{ext}"));
                            if p.is_file() {
                                return Some(p);
                            }
                        }
                    }
                }
            }
        }
        for ext in ["png", "svg", "xpm"] {
            let p = PathBuf::from("/usr/share/pixmaps").join(format!("{name}.{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(size: u32, p: Rgba8Pixel) -> Rgba {
        let mut b = Rgba::new(size, size);
        b.make_mut_slice().fill(p);
        b
    }

    #[test]
    fn composed_icon_puts_the_app_top_right() {
        let red = Rgba8Pixel {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let out = icon(Some(&solid(32, red)), 64);
        assert_eq!((out.width(), out.height()), (64, 64));
        let at = |x: u32, y: u32| out.as_slice()[(y * 64 + x) as usize];
        // Centre of the plate (top right) shows the app; the bottom-left shows the logo.
        let c = at(64 - 64 * 11 / 40, 64 * 11 / 40);
        assert!(c.r > 200 && c.g < 60, "{c:?}");
        assert_ne!(at(10, 54), c);
    }

    #[test]
    fn resize_keeps_transparent_edges_clean() {
        let mut b = Rgba::new(2, 1);
        b.make_mut_slice()[0] = Rgba8Pixel {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        let out = resize(&b, 1, 1);
        let p = out.as_slice()[0];
        assert_eq!(p.r, 255); // no dark fringe from the transparent black neighbour
        assert!((120..=135).contains(&p.a));
    }

    #[test]
    fn ico_and_icns_have_valid_headers() {
        let img = solid(
            16,
            Rgba8Pixel {
                r: 1,
                g: 2,
                b: 3,
                a: 255,
            },
        );
        let ico = encode_ico(&img);
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 5, 0]);
        let icns = encode_icns(&img);
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        assert!(decode_png(&encode_png(&img)).is_some());
    }

    #[test]
    fn profile_is_read_from_the_launch_command() {
        let id = "0123456789abcdef0123456789abcdef";
        // .lnk arguments (quoted), .desktop Exec, macOS launch script.
        for command in [
            format!("\"--launch\" \"{id}\" \"--open\" \"x\""),
            format!("Exec=/usr/bin/dpimech --launch {id} --open firefox"),
            format!("exec '/Applications/DPIMech.app/x' --launch '{id}'"),
        ] {
            assert_eq!(
                profile_after_flag(&command).as_deref(),
                Some(id),
                "{command}"
            );
        }
        assert_eq!(profile_after_flag("Exec=/usr/bin/dpimech"), None);
        assert_eq!(profile_after_flag("--launch \"\""), None);
    }

    #[test]
    fn names_are_safe_file_names() {
        assert_eq!(file_name("Discord: fast/slow?"), "Discord fastslow");
        assert_eq!(file_name("  ..  "), "");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_quoting_and_user_dirs() {
        assert_eq!(platform::desktop_quote("a b"), "\"a b\"");
        assert_eq!(platform::desktop_quote("$x%"), "\"\\\\$x%%\"");
        let home = Path::new("/home/u");
        assert_eq!(
            linux::parse_user_dirs("XDG_DESKTOP_DIR=\"$HOME/Masaüstü\"\n", home),
            Some(PathBuf::from("/home/u/Masaüstü"))
        );
        assert_eq!(
            linux::parse_user_dirs("XDG_DESKTOP_DIR=\"$HOME/\"\n", home),
            None
        );
    }
}
