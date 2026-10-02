//! Libraries the AppImage carries for systems that lack them (tools/build-linux-packages.sh puts
//! them next to the binary in `usr/lib`). Their users open them by name, so loading the bundled
//! files first by path lets that lookup find them. A system copy always wins.

use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};

/// libxkbcommon-x11: winit needs it for X11 windows and panics without it. Bare Arch (and
/// Wayland-only desktops running the app under XWayland) may not have it.
pub const XKB_X11: &[&str] = &["libxkbcommon-x11.so.0"];

/// libayatana-appindicator for the tray, dependencies first so each library finds the ones it
/// needs already loaded.
pub const INDICATOR: &[&str] = &[
    "libdbusmenu-glib.so.4",
    "libdbusmenu-gtk3.so.4",
    "libayatana-ido3-0.4.so.0",
    "libayatana-indicator3.so.7",
    "libayatana-appindicator3.so.1",
];

/// Loads `libs` from the AppImage unless the system has the last one (the one actually used).
pub fn load(libs: &[&str]) {
    let Some(name) = libs.last() else { return };
    // SAFETY: plain C libraries without constructors that depend on our state.
    if unsafe { Library::new(name) }.is_ok() {
        return;
    }
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.parent()?.join("lib")))
    else {
        return;
    };
    for name in libs {
        let path = dir.join(name);
        if !path.exists() {
            continue;
        }
        // SAFETY: as above. The libraries stay loaded for the life of the process.
        match unsafe { Library::open(Some(&path), RTLD_NOW | RTLD_GLOBAL) } {
            Ok(lib) => std::mem::forget(lib),
            Err(e) => eprintln!("bundled {name}: {e}"),
        }
    }
}
