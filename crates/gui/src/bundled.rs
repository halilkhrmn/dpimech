//! Libraries the AppImage carries for systems that lack them (tools/build-linux-packages.sh puts
//! them next to the binary in `usr/lib`). Their users open them by name, so loading the bundled
//! files first by path lets that lookup find them. A system copy always wins.

use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};

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

/// winit opens libxkbcommon-x11 for an X11 window and panics without it (bare Arch: it is a package of
/// its own). It cannot be bundled: it uses libxkbcommon's internals, so it must be the build that
/// matches the system's libxkbcommon (a copy from the build machine crashes on Arch). On Wayland it
/// is not needed.
pub fn x11_keyboard_missing() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::env::var_os("DISPLAY").is_some()
        // SAFETY: a plain C library; it is only probed and closed again.
        && unsafe { Library::new("libxkbcommon-x11.so.0") }.is_err()
}

/// Says which library to install, on stderr and in a GTK dialog (GTK works without it), instead of
/// a panic the user never sees.
pub fn report_x11_keyboard_missing() {
    use gtk::prelude::*;
    let text = crate::i18n::tr(
        "DPIMech needs the libxkbcommon-x11 library to open its window. Install it with your package manager (Arch: sudo pacman -S libxkbcommon-x11) and start DPIMech again.",
    );
    eprintln!("{text}");
    if gtk::init().is_err() {
        return;
    }
    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Close,
        &text,
    );
    dialog.set_title("DPIMech");
    dialog.run();
}
