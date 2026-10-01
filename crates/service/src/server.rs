//! Accepts GUI connections and dispatches requests to the supervisor.

use dpimech_core::ipc::{
    ClientFrame, Event, Listener, Reply, Request, ServerFrame, frame_lines, read_frame, write_frame,
};
use tokio::sync::{broadcast, mpsc};

use crate::supervisor::Supervisor;

pub async fn serve(supervisor: Supervisor, events: broadcast::Sender<Event>) -> anyhow::Result<()> {
    let mut listener = Listener::bind()?;
    supervisor.logs.info(
        "service",
        format!(
            "listening (data dir: {})",
            supervisor.data_dir().root.display()
        ),
    );
    loop {
        let stream = listener.accept().await?;
        let supervisor = supervisor.clone();
        let events = events.subscribe();
        tokio::spawn(async move {
            if let Err(e) = handle(stream, supervisor, events).await {
                tracing::debug!("client disconnected: {e}");
            }
        });
    }
}

async fn handle<S>(
    stream: S,
    supervisor: Supervisor,
    mut events: broadcast::Receiver<Event>,
) -> std::io::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<ServerFrame>();

    // Single writer task so replies and events never interleave mid-line.
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            if write_frame(&mut writer, &frame).await.is_err() {
                break;
            }
        }
    });

    let event_tx = out_tx.clone();
    let event_task = tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => {
                    if event_tx.send(ServerFrame::Event { event }).is_err() {
                        break;
                    }
                }
                // A slow client missed events; tell it to resync.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let _ = event_tx.send(ServerFrame::Event {
                        event: Event::ProfilesChanged,
                    });
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let mut lines = frame_lines(reader);
    let result = loop {
        match read_frame::<_, ClientFrame>(&mut lines).await {
            Ok(Some(frame)) => {
                let result = dispatch(&supervisor, frame.request)
                    .await
                    .map_err(|e| format!("{e:#}"));
                if out_tx
                    .send(ServerFrame::Reply {
                        id: frame.id,
                        result,
                    })
                    .is_err()
                {
                    break Ok(());
                }
            }
            Ok(None) => break Ok(()),
            Err(e) => break Err(e),
        }
    };
    event_task.abort();
    drop(out_tx);
    let _ = writer_task.await;
    result
}

async fn dispatch(supervisor: &Supervisor, request: Request) -> anyhow::Result<Reply> {
    Ok(match request {
        Request::Hello { .. } => Reply::Hello {
            service_version: dpimech_core::VERSION.to_owned(),
            data_dir: supervisor.data_dir().root.display().to_string(),
        },
        Request::ListProfiles => Reply::Profiles {
            profiles: supervisor.list().await,
        },
        Request::SaveProfile { profile } => {
            supervisor.save(profile).await?;
            Reply::Ok
        }
        Request::DeleteProfile { id } => {
            supervisor.delete(&id).await?;
            Reply::Ok
        }
        Request::StartProfile { id } => {
            supervisor.start(&id).await?;
            Reply::Ok
        }
        Request::StopProfile { id } => {
            supervisor.stop(&id).await?;
            Reply::Ok
        }
        Request::RecentLogs => Reply::Logs {
            lines: supervisor.logs.recent(),
        },
        Request::ListPackages => Reply::Packages {
            packages: supervisor.packages.list().await,
        },
        Request::CheckUpdates => {
            supervisor.packages.check_updates().await?;
            Reply::Ok
        }
        Request::InstallPackage { id } => {
            supervisor.install_package(id).await?;
            Reply::Ok
        }
        Request::LabStrategies { engine } => Reply::LabStrategies {
            strategies: supervisor.lab.strategies(engine),
        },
        Request::RefreshLabStrategies { engine } => Reply::LabStrategies {
            strategies: supervisor.lab.refresh(engine).await?,
        },
        Request::DetectIsp => Reply::Isp {
            info: supervisor.lab.detect_isp().await?,
        },
        Request::StartLab { request } => {
            supervisor.start_lab(request).await?;
            Reply::Ok
        }
        Request::CheckAppUpdate => Reply::AppUpdate {
            current: dpimech_core::VERSION.to_owned(),
            latest: supervisor.packages.check_app_update().await?,
            url: format!(
                "https://github.com/{}/releases/latest",
                dpimech_core::catalog::APP_REPO
            ),
        },
        Request::PrepareAppUpdate { format } => {
            let (version, path) = supervisor.packages.prepare_app_update(format).await?;
            Reply::AppUpdateReady {
                version,
                path: path.display().to_string(),
            }
        }
        Request::InstallAppUpdate => {
            #[cfg(windows)]
            supervisor.packages.install_app_update().await?;
            #[cfg(not(windows))]
            anyhow::bail!("the service installs updates only on Windows");
            #[cfg(windows)]
            Reply::Ok
        }
        Request::AddDefenderExclusion => {
            crate::defender::exclude(&supervisor.data_dir().root.join("engines")).await?;
            Reply::Ok
        }
        Request::DefenderStatus => Reply::DefenderStatus {
            excluded: crate::defender::is_excluded(&supervisor.data_dir().root.join("engines"))
                .await,
        },
        Request::ForeignTools => Reply::ForeignTools {
            tools: supervisor.foreign_tools().await,
        },
        Request::CheckProfile { id } => {
            supervisor.check_now(&id).await?;
            Reply::Ok
        }
        Request::SetDetailedLog { on } => {
            if on != supervisor.logs.detailed() {
                supervisor.logs.set_detailed(on);
                supervisor.logs.info(
                    "service",
                    if on {
                        "detailed log on"
                    } else {
                        "detailed log off"
                    },
                );
            }
            Reply::Ok
        }
        Request::CancelLab => {
            supervisor.lab.cancel().await;
            Reply::Ok
        }
        Request::RemovePackage { id } => {
            supervisor.remove_package(id).await?;
            Reply::Ok
        }
    })
}
