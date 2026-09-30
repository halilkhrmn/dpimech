//! Background thread that owns the IPC connection and pushes updates into the UI.

use crate::i18n::tr;
use std::time::Duration;

use dpimech_core::ipc::{Client, Event, Reply, Request};
use dpimech_core::lab::LabRequest;
use dpimech_core::model::{EngineKind, LogLine, Profile};
use dpimech_core::packages::{PackageId, PackageInfo};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use tokio::sync::mpsc;

use crate::{AppWindow, LogItem, PackageItem, Page, convert, labui, wizard};

const MAX_LOG_ROWS: usize = 1000;
/// How often to look for other DPI tools running next to DPIMech.
const FOREIGN_CHECK: Duration = Duration::from_secs(30);

pub enum Command {
    Save(Profile),
    Delete(String),
    Start(String),
    Stop(String),
    /// Hotkey: stop everything that runs, or bring back what ran last time.
    ToggleAll,
    CheckUpdates,
    InstallPackage(PackageId),
    RemovePackage(PackageId),
    LabStrategies(EngineKind),
    LabRefresh(EngineKind),
    DetectIsp,
    StartLab(LabRequest),
    CancelLab,
    /// Setup wizard: save a finished profile and switch it on.
    SaveAndStart(Profile),
    /// Run a profile's connection check now.
    CheckProfile(String),
    CheckAppUpdate,
    AddDefenderExclusion,
}

pub fn spawn(ui: Weak<AppWindow>, commands: mpsc::UnboundedReceiver<Command>) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        runtime.block_on(run(ui, commands));
    });
}

async fn run(ui: Weak<AppWindow>, mut commands: mpsc::UnboundedReceiver<Command>) {
    let mut last_active: Vec<String> = Vec::new();
    loop {
        let Ok((client, mut events)) = Client::connect().await else {
            set_connected(&ui, false, tr("Start it with: dpimech-service run"));
            // Drop commands issued while offline instead of replaying them later.
            while commands.try_recv().is_ok() {}
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        };

        let info = match client
            .request(Request::Hello {
                client_version: dpimech_core::VERSION.into(),
            })
            .await
        {
            Ok(Reply::Hello {
                service_version,
                data_dir,
            }) => trf!("Version {} · data in {}", service_version, data_dir),
            _ => String::new(),
        };
        set_connected(&ui, true, info);
        refresh_profiles(&ui, &client).await;
        refresh_packages(&ui, &client).await;
        if let Ok(Reply::DefenderStatus { excluded }) =
            client.request(Request::DefenderStatus).await
        {
            let _ = ui.upgrade_in_event_loop(move |ui| ui.set_defender_excluded(excluded));
        }
        if let Ok(Reply::Logs { lines }) = client.request(Request::RecentLogs).await {
            replace_logs(&ui, lines);
        }

        let mut foreign_check = tokio::time::interval(FOREIGN_CHECK);
        loop {
            tokio::select! {
                _ = foreign_check.tick() => refresh_foreign_tools(&ui, &client).await,
                Some(cmd) = commands.recv() => handle_command(&ui, &client, cmd, &mut last_active).await,
                event = events.recv() => match event {
                    Some(Event::Log { line }) => append_log(&ui, line),
                    Some(Event::ProfilesChanged | Event::ProfileStatus { .. }) => refresh_profiles(&ui, &client).await,
                    Some(Event::PackagesChanged) => refresh_packages(&ui, &client).await,
                    Some(Event::AppUpdateAvailable { version, url }) => {
                        crate::notify::app_update(&version);
                        let _ = ui.upgrade_in_event_loop(move |ui| {
                            crate::set_app_update(&ui, trf!("Version {} is available.", version), Some(url));
                        });
                    }
                    Some(Event::LabProgress { done, total, result }) => {
                        if let Some(result) = result {
                            let _ = ui.upgrade_in_event_loop(move |ui| {
                                wizard::on_lab_progress(&ui, done, total, &result);
                                labui::progress(&ui, done, total, result);
                            });
                        }
                    }
                    Some(Event::LabFinished { cancelled, error }) => {
                        let _ = ui.upgrade_in_event_loop(move |ui| {
                            wizard::on_lab_finished(&ui, cancelled, error.clone());
                            labui::finished(&ui, cancelled, error);
                        });
                    }
                    None => break,
                },
            }
        }
    }
}

async fn handle_command(
    ui: &Weak<AppWindow>,
    client: &Client,
    cmd: Command,
    last_active: &mut Vec<String>,
) {
    let (request, is_editor) = match cmd {
        Command::Save(profile) => (Request::SaveProfile { profile }, true),
        Command::Delete(id) => (Request::DeleteProfile { id }, true),
        Command::Start(id) => (Request::StartProfile { id }, false),
        Command::CheckProfile(id) => (Request::CheckProfile { id }, false),
        Command::CheckAppUpdate => {
            let text = match client.request(Request::CheckAppUpdate).await {
                Ok(Reply::AppUpdate {
                    current,
                    latest: Some(latest),
                    url,
                }) => Ok((
                    trf!("Version {} is available (you have {}).", latest, current),
                    Some(url),
                )),
                Ok(Reply::AppUpdate { current, .. }) => {
                    Ok((trf!("You have the latest version ({}).", current), None))
                }
                Ok(_) => Ok((String::new(), None)),
                Err(e) => Err(trf!("Could not check for updates: {}", e)),
            };
            let _ = ui.upgrade_in_event_loop(move |ui| match text {
                Ok((text, url)) => crate::set_app_update(&ui, text, url),
                Err(e) => crate::set_app_update(&ui, e, None),
            });
            return;
        }
        Command::AddDefenderExclusion => {
            let result = client.request(Request::AddDefenderExclusion).await;
            let _ = ui.upgrade_in_event_loop(move |ui| match result {
                Ok(_) => {
                    ui.set_defender_state("done".into());
                    // The check mark stays long enough to be seen, then the whole hint fades
                    // away: its job is done. Errors stay until the next try.
                    let weak = ui.as_weak();
                    slint::Timer::single_shot(Duration::from_secs(5), move || {
                        if let Some(ui) = weak.upgrade() {
                            ui.set_defender_state("hiding".into());
                            let weak = ui.as_weak();
                            slint::Timer::single_shot(Duration::from_millis(450), move || {
                                if let Some(ui) = weak.upgrade() {
                                    ui.set_defender_excluded(true);
                                    ui.set_defender_state(SharedString::new());
                                }
                            });
                        }
                    });
                }
                Err(e) => {
                    ui.set_defender_status(e.to_string().into());
                    ui.set_defender_state("error".into());
                }
            });
            return;
        }
        Command::Stop(id) => (Request::StopProfile { id }, false),
        Command::ToggleAll => return toggle_all(ui, client, last_active).await,
        Command::CheckUpdates => {
            let result = client.request(Request::CheckUpdates).await;
            let _ = ui.upgrade_in_event_loop(move |ui| ui.set_checking_updates(false));
            if let Err(e) = result {
                append_log(ui, gui_error(format!("update check failed: {e}")));
            }
            return;
        }
        Command::InstallPackage(id) => (Request::InstallPackage { id }, false),
        Command::RemovePackage(id) => (Request::RemovePackage { id }, false),
        Command::LabStrategies(engine) | Command::LabRefresh(engine) => {
            let refresh = matches!(cmd, Command::LabRefresh(_));
            let request = if refresh {
                Request::RefreshLabStrategies { engine }
            } else {
                Request::LabStrategies { engine }
            };
            match client.request(request).await {
                Ok(Reply::LabStrategies { strategies }) => {
                    let _ = ui.upgrade_in_event_loop(move |ui| {
                        wizard::on_strategies(&ui, &strategies);
                        labui::set_strategies(&ui, strategies);
                    });
                }
                Err(e) => lab_status(ui, trf!("Could not load strategies: {}", e)),
                _ => {}
            }
            return;
        }
        Command::DetectIsp => {
            match client.request(Request::DetectIsp).await {
                Ok(Reply::Isp { info }) => {
                    let _ = ui.upgrade_in_event_loop(move |ui| {
                        labui::set_isp(&ui, &info);
                    });
                }
                Err(e) => lab_status(ui, trf!("ISP lookup failed: {}", e)),
                _ => {}
            }
            return;
        }
        Command::StartLab(request) => {
            let shown = request.clone();
            match client.request(Request::StartLab { request }).await {
                Ok(_) => {
                    let _ = ui.upgrade_in_event_loop(move |ui| labui::started(&ui, &shown));
                }
                Err(e) => {
                    let message = e.to_string();
                    let shown = message.clone();
                    let _ = ui.upgrade_in_event_loop(move |ui| wizard::on_lab_error(&ui, &shown));
                    lab_status(ui, message);
                }
            }
            return;
        }
        Command::SaveAndStart(profile) => {
            let id = profile.id.clone();
            let mut result = client
                .request(Request::SaveProfile { profile })
                .await
                .map(|_| ());
            if result.is_ok() {
                result = client
                    .request(Request::StartProfile { id })
                    .await
                    .map(|_| ());
            }
            let result = result.map_err(|e| e.to_string());
            let _ = ui.upgrade_in_event_loop(move |ui| wizard::on_profile_saved(&ui, result));
            refresh_profiles(ui, client).await;
            return;
        }
        Command::CancelLab => {
            let _ = client.request(Request::CancelLab).await;
            return;
        }
    };
    let result = client.request(request).await;
    if let Err(e) = &result
        && !is_editor
    {
        append_log(ui, gui_error(e.to_string()));
    }
    let _ = ui.upgrade_in_event_loop(move |ui| match result {
        Ok(_) if is_editor => ui.set_page(Page::Dashboard),
        Err(e) if is_editor => ui.set_editor_error(e.to_string().into()),
        _ => {}
    });
    if !is_editor {
        // Resync so a rejected toggle snaps back.
        refresh_profiles(ui, client).await;
    }
}

fn lab_status(ui: &Weak<AppWindow>, text: String) {
    let _ = ui.upgrade_in_event_loop(move |ui| ui.set_lab_status(text.into()));
}

async fn toggle_all(ui: &Weak<AppWindow>, client: &Client, last_active: &mut Vec<String>) {
    let Ok(Reply::Profiles { profiles }) = client.request(Request::ListProfiles).await else {
        return;
    };
    let active: Vec<String> = profiles
        .iter()
        .filter(|p| p.status.is_active())
        .map(|p| p.profile.id.clone())
        .collect();
    if !active.is_empty() {
        for id in &active {
            let _ = client
                .request(Request::StopProfile { id: id.clone() })
                .await;
        }
        *last_active = active;
    } else {
        let targets: Vec<String> = if last_active.is_empty() {
            profiles
                .iter()
                .filter(|p| p.profile.autostart)
                .map(|p| p.profile.id.clone())
                .collect()
        } else {
            last_active.clone()
        };
        for id in targets {
            let _ = client.request(Request::StartProfile { id }).await;
        }
    }
    refresh_profiles(ui, client).await;
}

async fn refresh_profiles(ui: &Weak<AppWindow>, client: &Client) {
    if let Ok(Reply::Profiles { profiles }) = client.request(Request::ListProfiles).await {
        crate::notify::on_profiles(&profiles);
        let _ = ui.upgrade_in_event_loop(move |ui| crate::apply_profiles(&ui, profiles));
    }
}

async fn refresh_packages(ui: &Weak<AppWindow>, client: &Client) {
    if let Ok(Reply::Packages { packages }) = client.request(Request::ListPackages).await {
        let _ = ui.upgrade_in_event_loop(move |ui| apply_packages(&ui, &packages));
    }
}

fn apply_packages(ui: &AppWindow, packages: &[PackageInfo]) {
    wizard::on_packages(ui, packages);
    ui.set_show_defender_note(packages.iter().any(|p| {
        matches!(p.id, PackageId::Zapret | PackageId::GoodbyeDpi) && p.installed_version.is_some()
    }));
    let items: Vec<PackageItem> = packages.iter().map(convert::package_item).collect();
    ui.set_updates_available(packages.iter().any(PackageInfo::update_available));
    ui.set_packages(ModelRc::new(VecModel::from(items)));
}

fn set_connected(ui: &Weak<AppWindow>, connected: bool, info: String) {
    let _ = ui.upgrade_in_event_loop(move |ui| {
        ui.set_service_connected(connected);
        ui.set_service_info(info.into());
        if !connected {
            crate::apply_profiles(&ui, Vec::new());
            apply_packages(&ui, &[]);
        }
    });
}

fn gui_error(text: String) -> LogLine {
    LogLine {
        unix_ms: chrono::Utc::now().timestamp_millis() as u64,
        level: dpimech_core::model::LogLevel::Error,
        source: "app".into(),
        text,
    }
}

fn replace_logs(ui: &Weak<AppWindow>, lines: Vec<LogLine>) {
    let _ = ui.upgrade_in_event_loop(move |ui| {
        let items: Vec<LogItem> = lines
            .iter()
            .rev()
            .take(MAX_LOG_ROWS)
            .map(convert::log_item)
            .collect();
        ui.set_logs(ModelRc::new(VecModel::from(items)));
    });
}

async fn refresh_foreign_tools(ui: &Weak<AppWindow>, client: &Client) {
    let text = match client.request(Request::ForeignTools).await {
        Ok(Reply::ForeignTools { tools }) => tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };
    let _ = ui.upgrade_in_event_loop(move |ui| ui.set_foreign_tools(text.into()));
}

fn append_log(ui: &Weak<AppWindow>, line: LogLine) {
    crate::logfile::write(&line);
    let _ = ui.upgrade_in_event_loop(move |ui| {
        let logs = ui.get_logs();
        if let Some(model) = logs.as_any().downcast_ref::<VecModel<LogItem>>() {
            model.insert(0, convert::log_item(&line));
            if model.row_count() > MAX_LOG_ROWS {
                model.remove(MAX_LOG_ROWS);
            }
        }
    });
}
