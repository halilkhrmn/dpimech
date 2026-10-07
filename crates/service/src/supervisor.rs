//! Owns profiles and the engine processes that run them.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use dpimech_core::config::ServiceConfig;
use dpimech_core::ipc::Event;
use dpimech_core::model::{
    ConnectionHealth, EngineKind, LogLevel, Os, Profile, ProfileState, ProfileStatus, Routing,
    RoutingMode, SlowAction,
};
use dpimech_core::packages::PackageId;
use dpimech_core::paths::DataDir;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, Notify, broadcast, mpsc, oneshot};

use crate::health::{MonitorEvent, MonitorHistory};
use crate::lab::Lab;
use crate::logs::{LogBus, now_ms};
use crate::packages::Packages;
use crate::perapp::{PerAppRouter, Route};

const SERVICE_SOURCE: &str = "service";

/// Local SOCKS port of a proxy engine running for the whole computer (Linux tpws). Only one
/// whole-computer profile runs at a time (see `takes_all_traffic`), so one fixed port does.
const SYSTEM_WIDE_PROXY_PORT: u16 = 10880;

/// The engine's local proxy port: the profile's own, or the fixed one for a whole-computer
/// proxy engine.
fn engine_port(profile: &Profile) -> Option<u16> {
    profile.routing.port().or_else(|| {
        (profile.engine.is_proxy() && profile.routing.mode() == RoutingMode::SystemWide)
            .then_some(SYSTEM_WIDE_PROXY_PORT)
    })
}

/// Profiles whose traffic goes through the per-app router (ProxiFyre / Linux cgroups).
fn is_routed(profile: &Profile) -> bool {
    match profile.routing.mode() {
        RoutingMode::PerApp => true,
        RoutingMode::SystemWide => profile.engine.is_proxy(),
        RoutingMode::LocalProxy | RoutingMode::AppProxy => false,
    }
}

/// Whole-computer profiles (packet engines or a redirected proxy) cannot share the traffic.
fn takes_all_traffic(profile: &Profile) -> bool {
    profile.engine.intercepts_packets() || profile.routing.mode() == RoutingMode::SystemWide
}

struct Running {
    /// Which start this is: a stopped run that is still shutting down must not remove or
    /// overwrite the run that replaced it.
    run: u64,
    /// `Some(reason)` stops the engine and marks the profile as failed.
    stop: oneshot::Sender<Option<String>>,
    /// Wakes the connection monitor for an immediate check.
    check_now: Arc<Notify>,
}

struct State {
    config: ServiceConfig,
    status: HashMap<String, ProfileStatus>,
    running: HashMap<String, Running>,
    /// Profiles that failed only because their engine was missing; cleared once it is installed.
    missing_engine: HashSet<String>,
    next_run: u64,
}

#[derive(Clone)]
pub struct Supervisor {
    data: Arc<DataDir>,
    state: Arc<Mutex<State>>,
    pub logs: Arc<LogBus>,
    pub packages: Packages,
    pub lab: Lab,
    router: Arc<PerAppRouter>,
    router_failures: Arc<Mutex<Option<mpsc::UnboundedReceiver<String>>>>,
    events: broadcast::Sender<Event>,
}

/// Shown on the card when the first check after start fails; translated by the GUI.
const NOT_WORKING_ADVICE: &str = "This setting does not open this profile's sites on your connection. Find a working one in the Strategy Lab, or try another engine.";

impl Supervisor {
    pub fn new(data: DataDir, events: broadcast::Sender<Event>) -> anyhow::Result<Self> {
        let config = ServiceConfig::load(&data.config_file())?;
        let logs = Arc::new(LogBus::new(events.clone()));
        let data = Arc::new(data);
        let packages = Packages::new(data.clone(), events.clone(), logs.clone());
        let (failures_tx, failures_rx) = mpsc::unbounded_channel();
        let lab = Lab::new(data.clone(), packages.clone(), events.clone(), logs.clone());
        let router = Arc::new(PerAppRouter::new(
            packages.clone(),
            logs.clone(),
            failures_tx,
        ));
        Ok(Self {
            data,
            state: Arc::new(Mutex::new(State {
                config,
                status: HashMap::new(),
                running: HashMap::new(),
                missing_engine: HashSet::new(),
                next_run: 0,
            })),
            logs,
            packages,
            lab,
            router,
            router_failures: Arc::new(Mutex::new(Some(failures_rx))),
            events,
        })
    }

    /// Starts long-running housekeeping: router failure handling and daily update checks.
    pub async fn spawn_background(&self) {
        if let Some(mut failures) = self.router_failures.lock().await.take() {
            let this = self.clone();
            tokio::spawn(async move {
                while let Some(reason) = failures.recv().await {
                    this.logs.error("ProxiFyre", &reason);
                    this.fail_per_app_profiles(&reason).await;
                }
            });
        }
        let this = self.clone();
        let mut events = self.events.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                if matches!(event, Event::PackagesChanged) {
                    this.clear_resolved_missing_engines().await;
                }
            }
        });
        let packages = self.packages.clone();
        let lab = self.lab.clone();
        let events = self.events.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                if let Err(e) = packages.check_updates().await {
                    tracing::warn!("update check failed: {e:#}");
                }
                if let Err(e) = lab.update_standard_strategies().await {
                    tracing::warn!("{e:#}");
                }
                if let Err(e) = lab.update_domain_packs().await {
                    tracing::warn!("{e:#}");
                }
                if let Ok(Some(version)) = packages.check_app_update().await {
                    let _ = events.send(Event::AppUpdateAvailable {
                        url: format!(
                            "https://github.com/{}/releases/latest",
                            dpimech_core::catalog::APP_REPO
                        ),
                        version,
                    });
                }
                tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
            }
        });
    }

    pub fn data_dir(&self) -> &DataDir {
        &self.data
    }

    pub async fn start_autostart_profiles(&self) {
        let ids: Vec<String> = {
            let state = self.state.lock().await;
            state
                .config
                .profiles
                .iter()
                .filter(|p| p.autostart)
                .map(|p| p.id.clone())
                .collect()
        };
        for id in ids {
            if let Err(e) = self.start(&id).await {
                self.logs
                    .error(SERVICE_SOURCE, format!("autostart failed: {e:#}"));
            }
        }
    }

    pub async fn list(&self) -> Vec<ProfileState> {
        let state = self.state.lock().await;
        state
            .config
            .profiles
            .iter()
            .map(|p| ProfileState {
                profile: p.clone(),
                status: state
                    .status
                    .get(&p.id)
                    .cloned()
                    .unwrap_or(ProfileStatus::Stopped),
            })
            .collect()
    }

    pub async fn save(&self, mut profile: Profile) -> anyhow::Result<()> {
        validate(&profile, &self.data)?;
        if profile.id.is_empty() {
            profile.id = Profile::new_id();
        }
        let mut state = self.state.lock().await;
        if state.running.contains_key(&profile.id) {
            bail!("stop the profile before editing it");
        }
        match state
            .config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile.id)
        {
            Some(existing) => *existing = profile,
            None => state.config.profiles.push(profile),
        }
        state.config.save(&self.data.config_file())?;
        drop(state);
        let _ = self.events.send(Event::ProfilesChanged);
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.stop(id).await?;
        let mut state = self.state.lock().await;
        state.config.profiles.retain(|p| p.id != id);
        state.status.remove(id);
        state.config.save(&self.data.config_file())?;
        drop(state);
        let _ = self.events.send(Event::ProfilesChanged);
        Ok(())
    }

    pub async fn start(&self, id: &str) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        if state.running.contains_key(id) {
            return Ok(());
        }
        let profile = state
            .config
            .profiles
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .context("unknown profile")?;

        if let Some(conflict) = find_conflict(&state, &profile) {
            bail!("conflicts with running profile \"{conflict}\"");
        }
        if profile.engine.intercepts_packets() && self.lab.is_running().await {
            bail!("the Strategy Lab is running; wait for it to finish or cancel it");
        }

        let command = match self.build_command(&profile) {
            Ok(cmd) => cmd,
            Err(e) => {
                let message = format!("{e:#}");
                if self.engine_binary(&profile).is_none() {
                    state.missing_engine.insert(id.to_owned());
                }
                self.set_status(
                    &mut state,
                    id,
                    ProfileStatus::Error {
                        message: message.clone(),
                    },
                );
                self.logs.error(&profile.name, message);
                return Ok(());
            }
        };

        self.set_status(&mut state, id, ProfileStatus::Starting);
        {
            // Not a hard stop (the user may know what they do), but the first thing to check
            // when a profile "works but breaks things".
            let this = self.clone();
            let name = profile.name.clone();
            tokio::spawn(async move {
                for tool in this.foreign_tools().await {
                    this.logs.push(
                        LogLevel::Warn,
                        &name,
                        format!(
                            "{} is also running ({}); two DPI tools on the same traffic break connections",
                            tool.name, tool.path
                        ),
                    );
                }
            });
        }
        let (stop_tx, stop_rx) = oneshot::channel();
        let check_now = Arc::new(Notify::new());
        state.next_run += 1;
        let run = state.next_run;
        state.running.insert(
            id.to_owned(),
            Running {
                run,
                stop: stop_tx,
                check_now: check_now.clone(),
            },
        );
        drop(state);

        let this = self.clone();
        tokio::spawn(async move {
            this.run_process(profile, command, stop_rx, check_now, run)
                .await
        });
        Ok(())
    }

    /// Triggers the connection monitor of a running profile now ("Check now" in the UI).
    pub async fn check_now(&self, id: &str) -> anyhow::Result<()> {
        let state = self.state.lock().await;
        let running = state
            .running
            .get(id)
            .context("the profile is not running")?;
        let profile = state.config.profiles.iter().find(|p| p.id == id);
        if profile.is_none_or(|p| p.monitored_sites().is_empty()) {
            bail!("add sites to check to this profile first");
        }
        running.check_now.notify_one();
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> anyhow::Result<()> {
        let running = self.state.lock().await.running.remove(id);
        if let Some(running) = running {
            let _ = running.stop.send(None);
        }
        Ok(())
    }

    /// A "not installed" error is stale once the engine appears; reset those profiles to Stopped.
    async fn clear_resolved_missing_engines(&self) {
        let mut state = self.state.lock().await;
        let resolved: Vec<String> = state
            .missing_engine
            .iter()
            .filter(|id| {
                state
                    .config
                    .profiles
                    .iter()
                    .find(|p| &p.id == *id)
                    .is_none_or(|p| self.engine_binary(p).is_some())
            })
            .cloned()
            .collect();
        for id in resolved {
            state.missing_engine.remove(&id);
            if !state.running.contains_key(&id) {
                self.set_status(&mut state, &id, ProfileStatus::Stopped);
            }
        }
    }

    /// WinDivert engines act on the whole machine, so a test cannot share it with a profile.
    pub async fn start_lab(&self, request: dpimech_core::lab::LabRequest) -> anyhow::Result<()> {
        if request.engine.intercepts_packets() {
            let state = self.state.lock().await;
            if let Some(p) = state
                .config
                .profiles
                .iter()
                .find(|p| p.engine.intercepts_packets() && state.running.contains_key(&p.id))
            {
                bail!(
                    "stop the profile \"{}\" first: only one WinDivert engine can run",
                    p.name
                );
            }
        }
        self.lab.start(request).await
    }

    /// The driver cannot be replaced while ProxiFyre has it open: its installer then fails with
    /// 1603 after the old version is already gone.
    pub async fn install_package(&self, id: PackageId) -> anyhow::Result<()> {
        if id == PackageId::PacketFilterDriver {
            let users: Vec<String> = {
                let state = self.state.lock().await;
                state
                    .config
                    .profiles
                    .iter()
                    .filter(|p| state.running.contains_key(&p.id))
                    .filter(|p| p.routing.mode() == RoutingMode::PerApp)
                    .map(|p| p.name.clone())
                    .collect()
            };
            if !users.is_empty() {
                bail!(
                    "{} is in use by: {} — stop those profiles first",
                    id.display_name(),
                    users.join(", ")
                );
            }
        }
        self.packages.install(id).await
    }

    /// Refuses to delete a package that a running profile depends on.
    pub async fn remove_package(&self, id: PackageId) -> anyhow::Result<()> {
        let users: Vec<String> = {
            let state = self.state.lock().await;
            state
                .config
                .profiles
                .iter()
                .filter(|p| state.running.contains_key(&p.id))
                .filter(|p| {
                    PackageId::for_engine(p.engine) == Some(id)
                        || (id == PackageId::ProxiFyre && p.routing.mode() == RoutingMode::PerApp)
                })
                .map(|p| p.name.clone())
                .collect()
        };
        if !users.is_empty() {
            bail!(
                "{} is in use by: {} — stop those profiles first",
                id.display_name(),
                users.join(", ")
            );
        }
        self.packages.remove(id).await
    }

    /// DPI tools running outside DPIMech's engine folder.
    pub async fn foreign_tools(&self) -> Vec<dpimech_core::ipc::ForeignTool> {
        let engines = self.data.root.join("engines");
        tokio::task::spawn_blocking(move || crate::foreign::scan(&engines))
            .await
            .unwrap_or_default()
    }

    fn engine_binary(&self, profile: &Profile) -> Option<std::path::PathBuf> {
        self.packages.engine_path(profile.engine)
    }

    async fn fail_per_app_profiles(&self, reason: &str) {
        let mut state = self.state.lock().await;
        let per_app: Vec<String> = state
            .config
            .profiles
            .iter()
            .filter(|p| is_routed(p))
            .map(|p| p.id.clone())
            .collect();
        for id in per_app {
            if let Some(running) = state.running.remove(&id) {
                let _ = running.stop.send(Some(reason.to_owned()));
            }
        }
    }

    /// Points ProxiFyre at every per-app profile whose engine is currently running.
    async fn sync_routes(&self) -> anyhow::Result<()> {
        let routes: Vec<Route> = {
            let state = self.state.lock().await;
            state
                .config
                .profiles
                .iter()
                .filter(|p| {
                    state.running.contains_key(&p.id)
                        && matches!(state.status.get(&p.id), Some(ProfileStatus::Running { .. }))
                })
                .filter_map(|p| match &p.routing {
                    Routing::PerApp {
                        apps,
                        port,
                        tcp_only,
                    } => Some(Route {
                        apps: apps.clone(),
                        port: *port,
                        tcp_only: *tcp_only,
                        system_wide: None,
                        by_name: p.engine == EngineKind::SpoofDpi,
                    }),
                    Routing::SystemWide { .. } if is_routed(p) => Some(Route {
                        apps: Vec::new(),
                        port: engine_port(p)?,
                        tcp_only: true,
                        system_wide: Some(system_wide_ports(&p.args)),
                        by_name: false,
                    }),
                    _ => None,
                })
                .collect()
        };
        self.router.apply(routes).await
    }

    pub async fn stop_all(&self) {
        let running: Vec<Running> = self
            .state
            .lock()
            .await
            .running
            .drain()
            .map(|(_, r)| r)
            .collect();
        for r in running {
            let _ = r.stop.send(None);
        }
        if let Err(e) = self.router.apply(Vec::new()).await {
            tracing::warn!("stopping ProxiFyre: {e:#}");
        }
    }

    fn set_status(&self, state: &mut State, id: &str, status: ProfileStatus) {
        state.status.insert(id.to_owned(), status.clone());
        let _ = self.events.send(Event::ProfileStatus {
            id: id.to_owned(),
            status,
        });
    }

    fn build_command(&self, profile: &Profile) -> anyhow::Result<Command> {
        let domains: &[String] = match &profile.routing {
            Routing::SystemWide { domains } => domains,
            _ => &[],
        };
        crate::launch::build(
            &self.data,
            &self.packages,
            &crate::launch::Launch {
                engine: profile.engine,
                args: &profile.args,
                port: engine_port(profile),
                domains,
                hostlist_name: &format!("profile-{}", profile.id),
            },
        )
    }

    /// Runs the engine until the user stops it, restarting it when it crashes, stalls or its
    /// scheduled restart comes due. ProxiFyre keeps pointing at the same port throughout, so a
    /// restart only drops the connections that were open at that moment.
    async fn run_process(
        self,
        profile: Profile,
        first_command: Command,
        mut stop_rx: oneshot::Receiver<Option<String>>,
        check_now: Arc<Notify>,
        run: u64,
    ) {
        let id = profile.id.as_str();
        let name = profile.name.as_str();
        let per_app = is_routed(&profile);
        let auto_restart = profile.reliability.auto_restart;
        let schedule = (profile.reliability.restart_every_hours > 0).then(|| {
            Duration::from_secs(u64::from(profile.reliability.restart_every_hours) * 3600)
        });
        let since_unix = now_ms() / 1000;
        let mut restarts = 0u32;
        let mut crashes: VecDeque<Instant> = VecDeque::new();
        let mut command = Some(first_command);
        let monitor_sites = profile.monitored_sites();
        let monitor_every = profile.reliability.check_every_minutes;
        let monitor_history = Arc::new(std::sync::Mutex::new(MonitorHistory::default()));
        let mut health: Option<ConnectionHealth> = None;

        let final_status = 'run: loop {
            let mut cmd = match command
                .take()
                .map_or_else(|| self.build_command(&profile), Ok)
            {
                Ok(cmd) => cmd,
                Err(e) => {
                    break ProfileStatus::Error {
                        message: format!("{e:#}"),
                    };
                }
            };
            let _rules = match crate::launch::engine_rules(profile.engine, &cmd) {
                Ok(rules) => rules,
                Err(e) => {
                    break ProfileStatus::Error {
                        message: format!("{e:#}"),
                    };
                }
            };
            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(e) => {
                    // ERROR_ELEVATION_REQUIRED: WinDivert engines need the installed service.
                    let message = if e.raw_os_error() == Some(740) {
                        format!(
                            "{} needs administrator rights — run it through the installed DPIMech service",
                            profile.engine.display_name()
                        )
                    } else {
                        format!("failed to start {}: {e}", profile.engine.display_name())
                    };
                    break ProfileStatus::Error { message };
                }
            };
            crate::job::adopt(&child);

            let (stall_tx, mut stall_rx) = mpsc::channel::<String>(1);
            let markers = profile.engine.stall_markers();
            if let Some(out) = child.stdout.take() {
                self.pipe_output(name, out, markers, stall_tx.clone());
            }
            if let Some(err) = child.stderr.take() {
                self.pipe_output(name, err, markers, stall_tx.clone());
            }
            // The engine's stderr is block-buffered when piped, so "pool is full" can arrive
            // late or never; an active probe catches a stalled proxy within ~a minute.
            let probe = matches!(profile.engine, EngineKind::ByeDpi | EngineKind::ZapretTpws)
                .then(|| engine_port(&profile))
                .flatten()
                .map(|port| tokio::spawn(crate::health::probe_socks(port, stall_tx)));
            let _probe_guard = probe.map(AbortOnDrop);
            let (monitor_tx, mut monitor_rx) = mpsc::channel::<MonitorEvent>(4);
            let _monitor_guard = (monitor_every > 0 && !monitor_sites.is_empty()).then(|| {
                AbortOnDrop(tokio::spawn(crate::health::monitor_sites(
                    monitor_sites.clone(),
                    engine_port(&profile),
                    Duration::from_secs(u64::from(monitor_every) * 60),
                    monitor_history.clone(),
                    check_now.clone(),
                    monitor_tx,
                )))
            });
            if restarts == 0 {
                let args: Vec<String> = cmd
                    .as_std()
                    .get_args()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect();
                let port = engine_port(&profile)
                    .map(|p| format!(" on port {p}"))
                    .unwrap_or_default();
                self.logs.info(
                    name,
                    format!("{} started{port}", profile.engine.display_name()),
                );
                self.logs
                    .info(name, format!("strategy: {}", args.join(" ")));
            }
            {
                let mut state = self.state.lock().await;
                self.set_status(
                    &mut state,
                    id,
                    ProfileStatus::Running {
                        since_unix,
                        restarts,
                        health: health.clone(),
                    },
                );
            }
            if restarts == 0
                && per_app
                && let Err(e) = self.sync_routes().await
            {
                let _ = child.kill().await;
                break ProfileStatus::Error {
                    message: format!("{e:#}"),
                };
            }

            let started = Instant::now();
            let scheduled = sleep_or_forever(schedule);
            tokio::pin!(scheduled);
            let reason = loop {
                tokio::select! {
                    exit = child.wait() => break Restart::Exited(exit),
                    Some(line) = stall_rx.recv(), if auto_restart => {
                        // A stalled engine repeats the marker; give a fresh one time to settle.
                        if started.elapsed() >= STALL_GRACE {
                            break Restart::Stalled(line);
                        }
                    }
                    _ = &mut scheduled => break Restart::Scheduled,
                    Some(event) = monitor_rx.recv() => {
                        let (mut report, slow_reason) = match event {
                            MonitorEvent::First(mut h, failed) => {
                                if restarts == 0 {
                                    self.log_first_check(name, profile.engine, &mut h, &failed);
                                }
                                (h, None)
                            }
                            MonitorEvent::Report(h) => (h, None),
                            MonitorEvent::Slow(h, reason) => (h, Some(reason)),
                        };
                        let restart = match &slow_reason {
                            Some(reason) if profile.reliability.on_slow == SlowAction::Restart => {
                                if monitor_history.lock().unwrap().allow_restart() {
                                    Some(reason.clone())
                                } else {
                                    report.advice = Some(
                                        "Restarting did not help. This setting may have stopped working on your connection — run the Strategy Lab.".into(),
                                    );
                                    self.logs.push(LogLevel::Warn, name, format!("connection still slow ({reason}); automatic restarts paused"));
                                    None
                                }
                            }
                            Some(reason) => {
                                self.logs.push(LogLevel::Warn, name, format!("connection slow: {reason}"));
                                None
                            }
                            None => None,
                        };
                        health = Some(report);
                        {
                            let mut state = self.state.lock().await;
                            self.set_status(&mut state, id, ProfileStatus::Running {
                                since_unix,
                                restarts,
                                health: health.clone(),
                            });
                        }
                        if let Some(reason) = restart {
                            break Restart::Slow(reason);
                        }
                    }
                    stop = &mut stop_rx => {
                        let _ = child.kill().await;
                        break 'run stopped_status(stop);
                    }
                }
            };

            match reason {
                Restart::Exited(exit) => {
                    let what = match exit {
                        Ok(code) => format!("engine exited ({code})"),
                        Err(e) => format!("engine failed: {e}"),
                    };
                    if !auto_restart {
                        break ProfileStatus::Error { message: what };
                    }
                    let now = Instant::now();
                    crashes.push_back(now);
                    while crashes.front().is_some_and(|t| now - *t > CRASH_WINDOW) {
                        crashes.pop_front();
                    }
                    if crashes.len() > MAX_CRASHES {
                        break ProfileStatus::Error {
                            message: format!(
                                "{what}; gave up after {MAX_CRASHES} restarts in 5 minutes"
                            ),
                        };
                    }
                    let backoff = Duration::from_secs(crashes.len() as u64);
                    self.logs.push(
                        LogLevel::Warn,
                        name,
                        format!("{what}; restarting in {}s", backoff.as_secs()),
                    );
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => {}
                        stop = &mut stop_rx => break 'run stopped_status(stop),
                    }
                }
                Restart::Stalled(line) => {
                    self.logs.push(
                        LogLevel::Warn,
                        name,
                        format!("engine stalled (\"{line}\"); restarting it"),
                    );
                    let _ = child.kill().await;
                }
                Restart::Scheduled => {
                    self.logs.info(name, "scheduled restart");
                    let _ = child.kill().await;
                }
                Restart::Slow(reason) => {
                    self.logs.push(
                        LogLevel::Warn,
                        name,
                        format!("connection slow ({reason}); restarting the engine"),
                    );
                    let _ = child.kill().await;
                }
            }
            restarts += 1;
        };

        match &final_status {
            ProfileStatus::Error { message } => self.logs.error(name, message),
            _ => self.logs.info(name, "stopped"),
        }
        {
            let mut state = self.state.lock().await;
            // Stopped and started again quickly: the new run owns the entry and the status.
            let current = state.running.get(id).map(|r| r.run);
            if current == Some(run) {
                state.running.remove(id);
            }
            if current.is_none() || current == Some(run) {
                self.set_status(&mut state, id, final_status);
            }
        }
        if per_app && let Err(e) = self.sync_routes().await {
            self.logs.error("ProxiFyre", format!("{e:#}"));
        }
    }

    /// Forwards engine output to the log, collapsing floods of identical lines, and reports
    /// lines that match a stall marker.
    /// Says right after start whether the profile's sites open through the engine, so a setting
    /// that does not work on this connection shows up at once instead of as a silent failure.
    fn log_first_check(
        &self,
        name: &str,
        engine: EngineKind,
        health: &mut ConnectionHealth,
        failed: &[String],
    ) {
        let engine = engine.display_name();
        if failed.is_empty() {
            self.logs.info(
                name,
                format!(
                    "connection check: {}/{} site(s) open through {engine}, {} ms",
                    health.ok, health.total, health.latency_ms
                ),
            );
            return;
        }
        self.logs.warn(
            name,
            format!(
                "connection check: only {}/{} site(s) open through {engine} — failed: {}",
                health.ok,
                health.total,
                failed.join(", ")
            ),
        );
        if health.ok * 2 < health.total {
            health.advice = Some(NOT_WORKING_ADVICE.into());
        }
    }

    fn pipe_output(
        &self,
        source: &str,
        stream: impl AsyncRead + Unpin + Send + 'static,
        markers: &'static [&'static str],
        stall: mpsc::Sender<String>,
    ) {
        let logs = self.logs.clone();
        let source = source.to_owned();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            let mut last = String::new();
            let mut repeats = 0u32;
            while let Ok(Some(line)) = lines.next_line().await {
                let line = strip_ansi(&line).trim().to_owned();
                if line.is_empty() {
                    continue;
                }
                if markers.iter().any(|m| line.contains(m)) {
                    let _ = stall.try_send(line.clone());
                }
                if line == last {
                    repeats += 1;
                    continue;
                }
                if repeats > 0 {
                    logs.info(&source, format!("(previous line repeated {repeats}×)"));
                    repeats = 0;
                }
                logs.info(&source, &line);
                last = line;
            }
            if repeats > 0 {
                logs.info(&source, format!("(previous line repeated {repeats}×)"));
            }
        });
    }
}

/// Removes terminal colour codes (`ESC [ … letter`): SpoofDPI colours its log even when the
/// output is not a terminal, and the codes would show up as junk on the Logs page.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Stops the health probe when one engine run ends (restart or stop).
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

enum Restart {
    Exited(std::io::Result<std::process::ExitStatus>),
    Stalled(String),
    Scheduled,
    /// The connection monitor saw the profile's sites get slow or stop answering.
    Slow(String),
}

/// Ignore stall markers right after a (re)start; the old flood may still be in the pipe.
const STALL_GRACE: Duration = Duration::from_secs(30);
const CRASH_WINDOW: Duration = Duration::from_secs(5 * 60);
const MAX_CRASHES: usize = 5;

fn stopped_status(stop: Result<Option<String>, oneshot::error::RecvError>) -> ProfileStatus {
    match stop {
        Ok(Some(message)) => ProfileStatus::Error { message },
        _ => ProfileStatus::Stopped,
    }
}

async fn sleep_or_forever(duration: Option<Duration>) {
    match duration {
        Some(d) => tokio::time::sleep(d).await,
        None => std::future::pending().await,
    }
}

fn validate(profile: &Profile, data: &DataDir) -> anyhow::Result<()> {
    crate::launch::validate(data, profile.engine, &profile.args)
        .map_err(|e| anyhow::anyhow!("strategy arguments: {e}"))?;
    if let Routing::SystemWide { domains } = &profile.routing
        && domains.is_empty()
        && dpimech_core::catalog::uses_hostlist(&profile.args)
    {
        bail!("add the sites this profile should unblock (e.g. the Discord pack)");
    }
    if profile.name.trim().is_empty() {
        bail!("profile name is required");
    }
    let os = Os::current();
    if !profile.engine.supported_os().contains(&os) {
        bail!(
            "{} is not available on {}",
            profile.engine.display_name(),
            os.display_name()
        );
    }
    if !profile
        .engine
        .routing_modes(os)
        .contains(&profile.routing.mode())
    {
        bail!(
            "{} does not support {} routing",
            profile.engine.display_name(),
            profile.routing.mode().display_name()
        );
    }
    if let Routing::PerApp { apps, .. } = &profile.routing {
        if apps.is_empty() {
            bail!("select at least one application");
        }
    }
    // The service never runs this app (each user's GUI opens it), but it is stored and shown.
    if let Routing::AppProxy { app, .. } = &profile.routing
        && (app.trim().is_empty() || app.len() > 1024 || app.chars().any(char::is_control))
    {
        bail!("choose the application to open");
    }
    Ok(())
}

/// Returns the name of a running profile that cannot coexist with `candidate`.
fn find_conflict(state: &State, candidate: &Profile) -> Option<String> {
    state
        .config
        .profiles
        .iter()
        .filter(|p| p.id != candidate.id && state.running.contains_key(&p.id))
        .find(|p| {
            let port_clash = engine_port(p).is_some() && engine_port(p) == engine_port(candidate);
            let packet_clash = takes_all_traffic(p) && takes_all_traffic(candidate);
            let app_clash = match (&p.routing, &candidate.routing) {
                (Routing::PerApp { apps: a, .. }, Routing::PerApp { apps: b, .. }) => a
                    .iter()
                    .any(|x| b.iter().any(|y| x.eq_ignore_ascii_case(y))),
                _ => false,
            };
            port_clash || packet_clash || app_clash
        })
        .map(|p| p.name.clone())
}

/// TCP ports a whole-computer proxy engine handles: the strategy's `--filter-tcp`, else web.
fn system_wide_ports(args: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        let args = dpimech_core::args::split_args(args);
        if let Ok(Some(ports)) = crate::nfqueue::ports(&args, "--filter-tcp") {
            return ports;
        }
    }
    let _ = args;
    "80, 443".to_owned()
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;

    #[test]
    fn colour_codes_are_removed() {
        let line = "\u{1b}[32mINF\u{1b}[0m \u{1b}[90m2026-09-30T20:23:03Z\u{1b}[0m [app] \u{1b}[1mspoofdpi;\u{1b}[0m";
        assert_eq!(strip_ansi(line), "INF 2026-09-30T20:23:03Z [app] spoofdpi;");
        assert_eq!(strip_ansi("plain [x]"), "plain [x]");
    }
}
