#[cfg(windows)]
pub use windows::{Listener, connect};

#[cfg(unix)]
pub use unix::{Listener, connect};

#[cfg(windows)]
mod windows {
    use std::io;
    use std::sync::OnceLock;
    use std::time::Duration;

    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
    };
    use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;

    /// `DPIMECH_PIPE` lets a development service run next to the installed one.
    fn pipe_name() -> String {
        let name = std::env::var("DPIMECH_PIPE").unwrap_or_else(|_| "dpimech".into());
        format!(r"\\.\pipe\{name}")
    }

    /// SYSTEM and Administrators get full access; interactive users may read/write
    /// so the unelevated GUI can talk to the service running as SYSTEM.
    const PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;AU)";

    /// Security descriptor is created once and intentionally never freed.
    fn security_descriptor() -> io::Result<usize> {
        static SD: OnceLock<usize> = OnceLock::new();
        if let Some(sd) = SD.get() {
            return Ok(*sd);
        }
        let wide: Vec<u16> = PIPE_SDDL.encode_utf16().chain(Some(0)).collect();
        let mut sd = std::ptr::null_mut();
        // SAFETY: `wide` is a valid NUL-terminated UTF-16 string and `sd` is a valid out pointer.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(*SD.get_or_init(|| sd as usize))
    }

    fn create(first: bool) -> io::Result<NamedPipeServer> {
        let mut attrs = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security_descriptor()? as *mut _,
            bInheritHandle: 0,
        };
        let mut opts = ServerOptions::new();
        opts.first_pipe_instance(first).reject_remote_clients(true);
        // SAFETY: `attrs` points to a valid SECURITY_ATTRIBUTES for the duration of the call.
        unsafe {
            opts.create_with_security_attributes_raw(pipe_name(), &mut attrs as *mut _ as *mut _)
        }
    }

    pub struct Listener {
        next: NamedPipeServer,
    }

    impl Listener {
        /// Fails if another service instance already owns the pipe.
        pub fn bind() -> io::Result<Self> {
            Ok(Self {
                next: create(true)?,
            })
        }

        pub async fn accept(&mut self) -> io::Result<NamedPipeServer> {
            self.next.connect().await?;
            let fresh = create(false)?;
            Ok(std::mem::replace(&mut self.next, fresh))
        }
    }

    pub async fn connect() -> io::Result<NamedPipeClient> {
        for _ in 0..20 {
            match ClientOptions::new().open(pipe_name()) {
                Ok(client) => return Ok(client),
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(io::ErrorKind::TimedOut, "service pipe busy"))
    }
}

#[cfg(unix)]
mod unix {
    use std::io;
    use std::path::PathBuf;

    use tokio::net::{UnixListener, UnixStream};

    pub fn socket_path() -> PathBuf {
        std::env::var_os("DPIMECH_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                // macOS has no /run; /var/run is the traditional place there.
                if cfg!(target_os = "macos") {
                    PathBuf::from("/var/run/dpimech/dpimech.sock")
                } else {
                    PathBuf::from("/run/dpimech/dpimech.sock")
                }
            })
    }

    pub struct Listener {
        inner: UnixListener,
    }

    impl Listener {
        /// Fails if another service instance is already listening.
        pub fn bind() -> io::Result<Self> {
            use std::os::unix::fs::PermissionsExt;

            let path = socket_path();
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            // Removing the file of a live socket would silently take over its clients.
            if std::os::unix::net::UnixStream::connect(&path).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("another service is already listening on {}", path.display()),
                ));
            }
            let _ = std::fs::remove_file(&path);
            let inner = UnixListener::bind(&path)?;
            // Like the Windows pipe DACL: every local user may talk to the service, so every
            // request is validated as coming from an unprivileged client.
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666))?;
            Ok(Self { inner })
        }

        pub async fn accept(&mut self) -> io::Result<UnixStream> {
            Ok(self.inner.accept().await?.0)
        }
    }

    pub async fn connect() -> io::Result<UnixStream> {
        UnixStream::connect(socket_path()).await
    }
}
