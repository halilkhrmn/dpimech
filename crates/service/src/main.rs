//! DPIMech background service: runs DPI engines with the privileges they need
//! and exposes them to the unprivileged GUI over IPC.
//!
//! Usage:
//!   dpimech-service run [--data-dir <path>]   run in the console (development)
//!   dpimech-service install | uninstall       register with the OS service manager
//!                                             (Windows SCM, systemd on Linux, launchd on macOS)
//!   dpimech-service service                   entry point used by the service manager
//!   dpimech-service unit --exe <path>         print the systemd unit (used to build packages)

mod acl;
mod defender;
#[cfg(windows)]
mod firewall;
mod foreign;
mod health;
mod job;
mod lab;
mod launch;
#[cfg(target_os = "macos")]
mod launchd;
mod logs;
#[cfg(any(windows, target_os = "linux"))]
mod migrate;
#[cfg(target_os = "linux")]
mod nfqueue;
mod packages;
mod perapp;
#[cfg(target_os = "linux")]
mod perapp_linux;
mod server;
mod supervisor;
#[cfg(target_os = "linux")]
mod systemd;
#[cfg(windows)]
mod winsvc;

use std::path::PathBuf;

use dpimech_core::paths::{DataDir, default_service_data_dir};
use tokio::sync::broadcast;

use crate::supervisor::Supervisor;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "run".into());
    let mut data_dir = default_service_data_dir();
    let mut exe: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => {
                data_dir = PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--data-dir needs a path"))?,
                )
            }
            "--exe" => {
                exe = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--exe needs a path"))?,
                ))
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }
    let data = DataDir::new(data_dir);
    #[cfg(not(target_os = "linux"))]
    let _ = exe; // only the Linux `unit` command takes --exe

    match command.as_str() {
        "run" => {
            init_tracing(None);
            run_until(data, shutdown_signal())
        }
        #[cfg(windows)]
        "install" => winsvc::install(&data),
        #[cfg(windows)]
        "uninstall" => winsvc::uninstall(),
        #[cfg(windows)]
        "service" => {
            std::fs::create_dir_all(data.logs_dir())?;
            init_tracing(Some(data.logs_dir().join("service.log")));
            winsvc::run_dispatcher(data)
        }
        #[cfg(target_os = "linux")]
        "install" => systemd::install(&data),
        #[cfg(target_os = "linux")]
        "uninstall" => systemd::uninstall(),
        #[cfg(target_os = "macos")]
        "install" => launchd::install(&data),
        #[cfg(target_os = "macos")]
        "uninstall" => launchd::uninstall(),
        #[cfg(target_os = "macos")]
        "service" => {
            // stdout/stderr go to the log file named in the launchd plist.
            init_tracing(None);
            if let Err(e) = acl::harden_data_dir(&data.root) {
                tracing::error!("could not secure {}: {e:#}", data.root.display());
            }
            run_until(data, shutdown_signal())
        }
        #[cfg(target_os = "linux")]
        "unit" => {
            let exe = exe.ok_or_else(|| anyhow::anyhow!("unit needs --exe <path>"))?;
            print!("{}", systemd::unit_file(&exe, &data.root));
            Ok(())
        }
        #[cfg(target_os = "linux")]
        "service" => {
            // stdout goes to the journal.
            init_tracing(None);
            // Re-applied on every start in case someone loosened the permissions.
            if let Err(e) = acl::harden_data_dir(&data.root) {
                tracing::error!("could not secure {}: {e:#}", data.root.display());
            }
            run_until(data, shutdown_signal())
        }
        other => anyhow::bail!("unknown command: {other}"),
    }
}

/// Runs the service until `shutdown` resolves, then stops every engine.
pub fn run_until(
    data: DataDir,
    shutdown: impl std::future::Future<Output = ()>,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(&data.root)?;
    // A crashed service may have left nfqws' queue rules behind (harmless thanks to
    // `bypass`, but stale).
    #[cfg(target_os = "linux")]
    if is_root() {
        nfqueue::remove();
        perapp_linux::remove_leftovers();
    }
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let (events, _) = broadcast::channel(256);
        let supervisor = Supervisor::new(data, events.clone())?;
        supervisor.spawn_background().await;
        supervisor.start_autostart_profiles().await;

        let result = tokio::select! {
            r = server::serve(supervisor.clone(), events) => r,
            _ = shutdown => Ok(()),
        };
        supervisor.stop_all().await;
        // Give engines a moment to exit after the kill signal.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        result
    })
}

#[cfg(target_os = "linux")]
fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() == 0 }
}

/// Ctrl+C, or SIGTERM from the service manager / `kill`.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn init_tracing(log_file: Option<PathBuf>) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match log_file.and_then(|p| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .ok()
    }) {
        Some(file) => builder
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .init(),
        None => builder.init(),
    }
}
