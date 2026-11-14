//! Per-app routing through ProxiFyre (Windows). Linux uses cgroups + nftables instead
//! (`perapp_linux.rs`) behind the same `PerAppRouter::apply` interface.
//!
//! One ProxiFyre process serves every running per-app profile: each profile becomes one
//! routing rule pointing at its engine's local SOCKS5 port. Whenever the set of routes
//! changes, the config is rewritten and ProxiFyre is restarted.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub apps: Vec<String>,
    pub port: u16,
    pub tcp_only: bool,
    /// Linux: route all outgoing TCP to these ports (nftables set body, e.g. "80, 443")
    /// instead of only `apps` — tpws for the whole computer.
    pub system_wide: Option<String>,
    /// Linux: hand the engine the TLS server name instead of the address the app connected
    /// to, so an engine with its own DNS (SpoofDPI over DoH) gets around DNS blocking too.
    pub by_name: bool,
}

#[cfg(target_os = "linux")]
pub use crate::perapp_linux::PerAppRouter;
#[cfg(not(target_os = "linux"))]
pub use proxifyre::PerAppRouter;

#[cfg(not(target_os = "linux"))]
mod proxifyre {
    use super::Route;

    const SOURCE: &str = "ProxiFyre";
    use std::process::Stdio;
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::{Context, bail};
    use dpimech_core::model::EngineKind;
    use dpimech_core::packages::PackageId;
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;
    use tokio::sync::{Mutex, mpsc, oneshot};
    use tokio::task::JoinHandle;

    use crate::logs::LogBus;
    use crate::packages::{Packages, driver_version};

    struct Active {
        routes: Vec<Route>,
        stop: oneshot::Sender<()>,
        done: JoinHandle<()>,
    }

    pub struct PerAppRouter {
        packages: Packages,
        logs: Arc<LogBus>,
        active: Mutex<Option<Active>>,
        /// Receives a message when ProxiFyre dies on its own.
        failures: mpsc::UnboundedSender<String>,
    }

    impl PerAppRouter {
        pub fn new(
            packages: Packages,
            logs: Arc<LogBus>,
            failures: mpsc::UnboundedSender<String>,
        ) -> Self {
            Self {
                packages,
                logs,
                active: Mutex::new(None),
                failures,
            }
        }

        /// Makes ProxiFyre serve exactly `routes`. An empty list stops it.
        pub async fn apply(&self, mut routes: Vec<Route>) -> anyhow::Result<()> {
            routes.sort_by_key(|r| r.port);
            // Held for the whole call so concurrent profile changes restart ProxiFyre in order.
            let mut active = self.active.lock().await;
            if active.as_ref().map(|a| &a.routes) == Some(&routes) {
                return Ok(());
            }
            if let Some(old) = active.take() {
                let _ = old.stop.send(());
                let _ = old.done.await;
            }
            if routes.is_empty() {
                return Ok(());
            }

            if driver_version().is_none() {
                bail!("Windows Packet Filter driver is not installed — install it from Engines");
            }
            let exe = self
                .packages
                .binary_path(PackageId::ProxiFyre)
                .context("ProxiFyre is not installed — install it from Engines")?;
            let dir = exe.parent().context("ProxiFyre path has no parent")?;
            #[cfg(windows)]
            crate::firewall::allow_proxifyre(&exe);
            write_config(&dir.join("app-config.json"), &routes)?;

            let mut cmd = Command::new(&exe);
            cmd.current_dir(dir)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            if !is_elevated() {
                // Development runs are unelevated; ProxiFyre supports a limited mode for that.
                cmd.arg("--allow-not-admin");
            }
            #[cfg(windows)]
            cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);

            let mut child = cmd.spawn().context("starting ProxiFyre")?;
            crate::job::adopt(&child);
            for stream in [
                child
                    .stdout
                    .take()
                    .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
                child
                    .stderr
                    .take()
                    .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
            ]
            .into_iter()
            .flatten()
            {
                let logs = self.logs.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stream).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if !line.trim().is_empty() {
                            logs.info(SOURCE, line);
                        }
                    }
                });
            }

            // ProxiFyre fails fast when the driver or config is unusable.
            if let Ok(exit) = tokio::time::timeout(Duration::from_millis(1500), child.wait()).await
            {
                bail!("ProxiFyre exited right after starting ({})", exit?);
            }
            self.logs
                .info(SOURCE, format!("routing {} rule(s)", routes.len()));

            let (stop_tx, stop_rx) = oneshot::channel();
            let failures = self.failures.clone();
            let logs = self.logs.clone();
            let done = tokio::spawn(async move {
                tokio::select! {
                    exit = child.wait() => {
                        let reason = match exit {
                            Ok(code) => format!("ProxiFyre stopped unexpectedly ({code})"),
                            Err(e) => format!("ProxiFyre failed: {e}"),
                        };
                        let _ = failures.send(reason);
                    }
                    _ = stop_rx => stop(&mut child, &logs).await,
                }
            });
            *active = Some(Active {
                routes,
                stop: stop_tx,
                done,
            });
            Ok(())
        }
    }

    /// How long ProxiFyre gets to leave by itself. Short: Windows gives a service about five
    /// seconds at shutdown.
    #[cfg(windows)]
    const EXIT_GRACE: Duration = Duration::from_secs(3);

    /// Asks ProxiFyre to exit (Ctrl+C, what it expects) so it hands the adapters back to the
    /// packet filter driver itself, and only ends it when it does not leave in time.
    async fn stop(child: &mut tokio::process::Child, logs: &LogBus) {
        #[cfg(windows)]
        if let Some(pid) = child.id() {
            let asked = tokio::task::spawn_blocking(move || ctrl_c::send(pid))
                .await
                .unwrap_or(false);
            if asked {
                if tokio::time::timeout(EXIT_GRACE, child.wait()).await.is_ok() {
                    logs.info(SOURCE, "stopped");
                    return;
                }
                logs.warn(SOURCE, "did not exit when asked; ended it");
            }
        }
        let _ = logs;
        let _ = child.kill().await;
    }

    /// Ctrl+C for a child that has its own hidden console (`CREATE_NO_WINDOW`): the event only
    /// reaches processes on the caller's console, so the service joins the child's for a moment.
    #[cfg(windows)]
    mod ctrl_c {
        use std::sync::Mutex;
        use std::time::Duration;

        use windows_sys::Win32::System::Console::{
            AttachConsole, CTRL_C_EVENT, FreeConsole, GenerateConsoleCtrlEvent, GetConsoleWindow,
            GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
            SetConsoleCtrlHandler, SetStdHandle,
        };

        const STD_HANDLES: [u32; 3] = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE];

        /// A process has one console at a time.
        static CONSOLE: Mutex<()> = Mutex::new(());

        /// False when the event could not be sent; the caller then ends the process.
        pub fn send(pid: u32) -> bool {
            let _one_at_a_time = CONSOLE.lock().unwrap();
            // SAFETY: plain Win32 calls; every one is checked or harmless when it fails.
            unsafe {
                // A development run has a console of its own and would have to give it up.
                // The installed service has none.
                if !GetConsoleWindow().is_null() {
                    return false;
                }
                // Attaching points the service's empty standard handles at the child's console
                // and leaving does not clear them: every later process started with inherited
                // handles (the update installer) then fails with "The handle is invalid".
                let std_handles = STD_HANDLES.map(|id| GetStdHandle(id));
                if AttachConsole(pid) == 0 {
                    return false;
                }
                // The event goes to everyone on that console, which now includes us.
                SetConsoleCtrlHandler(None, 1);
                let sent = GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0) != 0;
                FreeConsole();
                for (id, handle) in STD_HANDLES.into_iter().zip(std_handles) {
                    SetStdHandle(id, handle);
                }
                // Delivery is asynchronous: stay deaf until it has passed.
                std::thread::sleep(Duration::from_millis(200));
                SetConsoleCtrlHandler(None, 0);
                sent
            }
        }
    }

    fn write_config(path: &std::path::Path, routes: &[Route]) -> anyhow::Result<()> {
        let proxies: Vec<_> = routes
        .iter()
        .map(|r| {
            json!({
                "appNames": r.apps,
                "socks5ProxyEndpoint": format!("127.0.0.1:{}", r.port),
                "socks5Transport": "TCP",
                "supportedProtocols": if r.tcp_only { json!(["TCP"]) } else { json!(["TCP", "UDP"]) },
                "supportedAddressFamilies": ["IPv4", "IPv6"],
            })
        })
        .collect();
        // Engines must never be routed into themselves.
        let excludes: Vec<&str> = EngineKind::ALL.iter().map(|e| e.binary_stem()).collect();
        let config = json!({
            "logLevel": "Error",
            "bypassLan": true,
            "proxies": proxies,
            "excludes": excludes,
        });
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&config)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    #[cfg(windows)]
    fn is_elevated() -> bool {
        // SAFETY: no arguments; returns a BOOL.
        unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
    }

    #[cfg(not(windows))]
    fn is_elevated() -> bool {
        true
    }
}
