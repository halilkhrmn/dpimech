//! Only one GUI per user session. A second launch (Start menu, sign-in entry, installer)
//! asks the running one to show its window and exits, instead of adding a second tray icon
//! and a second hotkey registration.

/// Returns `true` if this process is the first instance; `on_show` then runs whenever a
/// later launch asks for the window. Returns `false` after signalling the existing instance.
#[cfg(windows)]
pub fn acquire(on_show: impl Fn() + Send + 'static) -> bool {
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, EVENT_MODIFY_STATE, INFINITE, OpenEventW, SetEvent,
        WaitForSingleObject,
    };

    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let mutex_name = wide(r"Local\dpimech-gui");
    let event_name = wide(r"Local\dpimech-gui-show");

    // SAFETY: plain Win32 calls with NUL-terminated names; handles intentionally live for
    // the whole process (the mutex marks us as the running instance).
    unsafe {
        let mutex = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
        if !mutex.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            let event = OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr());
            if !event.is_null() {
                SetEvent(event);
            }
            return false;
        }
        // Auto-reset event: each SetEvent wakes the waiter once.
        let event = CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr());
        if !event.is_null() {
            let event = event as usize;
            std::thread::spawn(move || {
                loop {
                    WaitForSingleObject(event as _, INFINITE);
                    on_show();
                }
            });
        }
    }
    true
}

/// A Unix socket in the user's private runtime directory: connecting succeeds only while
/// the first instance listens, and any byte from a later launch means "show the window".
#[cfg(unix)]
pub fn acquire(on_show: impl Fn() + Send + 'static) -> bool {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    let Some(path) = socket_path() else {
        return true;
    };
    if let Ok(mut running) = UnixStream::connect(&path) {
        let _ = running.write_all(b"show\n");
        return false;
    }
    // Nobody answered: the file is left over from a crash.
    let _ = std::fs::remove_file(&path);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match UnixListener::bind(&path) {
        Ok(listener) => {
            std::thread::spawn(move || {
                for mut stream in listener.incoming().flatten() {
                    let mut byte = [0u8; 1];
                    if stream.read(&mut byte).is_ok_and(|n| n > 0) {
                        on_show();
                    }
                }
            });
        }
        Err(e) => eprintln!("single instance socket {}: {e}", path.display()),
    }
    true
}

#[cfg(unix)]
fn socket_path() -> Option<std::path::PathBuf> {
    // $XDG_RUNTIME_DIR is per user and mode 0700; ~/.cache is the fallback without a session.
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
        .map(|dir| dir.join("dpimech").join("gui.sock"))
}
