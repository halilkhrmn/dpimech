//! First-run setup wizard (Easy mode). Runs on the UI thread and is driven by events the
//! bridge forwards: package list updates, strategy lists, Lab progress and profile saves.

use crate::i18n::tr;
use std::cell::RefCell;

use dpimech_core::catalog::DOMAIN_PACKS;
use dpimech_core::lab::{LabRequest, LabResult, LabStrategy};
use dpimech_core::model::{EngineKind, Os, Profile, Reliability, Routing, RoutingMode};
use dpimech_core::packages::{PackageId, PackageInfo, PackageTask};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use tokio::sync::mpsc::UnboundedSender;

use crate::bridge::Command;
use crate::convert::parse_domains;
use crate::state::PROFILES;
use crate::{AppWindow, LabPack, WizardTask, picker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Where {
    Apps,
    WholeComputer,
    Proxy,
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Idle,
    Installing,
    LoadingStrategies,
    Testing,
    Saving,
    Done,
    Failed,
}

struct State {
    selected: Vec<bool>,
    where_: Where,
    phase: Phase,
    packages: Vec<PackageInfo>,
    requested: Vec<PackageId>,
    best: Option<LabResult>,
    baseline_ok: bool,
    profile_name: String,
    port: u16,
    /// Share of the Lab run finished (0..1).
    lab_done: f32,
    /// Stage that was running when the run failed, so its task shows ✕.
    failed_at: Option<Phase>,
    /// Errors of the strategies tried; if every one failed the same way, that is the story.
    strategy_errors: Vec<Option<String>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            selected: DOMAIN_PACKS.iter().map(|p| p.id == "discord").collect(),
            where_: default_where(),
            phase: Phase::Idle,
            packages: Vec::new(),
            requested: Vec::new(),
            best: None,
            baseline_ok: false,
            profile_name: String::new(),
            port: 1080,
            lab_done: 0.0,
            failed_at: None,
            strategy_errors: Vec::new(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static TX: RefCell<Option<UnboundedSender<Command>>> = const { RefCell::new(None) };
}

fn send(cmd: Command) {
    TX.with_borrow(|tx| {
        if let Some(tx) = tx {
            let _ = tx.send(cmd);
        }
    });
}

/// Per-app routing needs a redirector (ProxiFyre on Windows); where there is none yet, the
/// wizard does not offer it.
fn apps_supported() -> bool {
    EngineKind::ByeDpi
        .routing_modes(Os::current())
        .contains(&RoutingMode::PerApp)
}

/// A packet engine for the whole computer exists on Windows (winws) and Linux (nfqws).
fn whole_supported() -> bool {
    Os::current() != Os::MacOs
}

fn default_where() -> Where {
    if apps_supported() {
        Where::Apps
    } else if whole_supported() {
        Where::WholeComputer
    } else {
        Where::Proxy
    }
}

/// The proxy engine for "only when I choose": ByeDPI, or tpws on macOS where zapret's
/// release carries a build and ByeDPI's may not.
fn proxy_engine() -> EngineKind {
    if Os::current() == Os::MacOs {
        EngineKind::ZapretTpws
    } else {
        EngineKind::ByeDpi
    }
}

fn where_index(w: Where) -> i32 {
    match w {
        Where::Apps => 0,
        Where::WholeComputer => 1,
        Where::Proxy => 2,
    }
}

pub fn init(ui: &AppWindow, tx: UnboundedSender<Command>) {
    TX.set(Some(tx));
    refresh_packs(ui);
    ui.set_wizard_apps_supported(apps_supported());
    ui.set_wizard_whole_supported(whole_supported());
    ui.set_wizard_where(where_index(default_where()));
}

/// Opens the wizard at the welcome question (first run) or directly at the sites step.
pub fn open(ui: &AppWindow, from_welcome: bool) {
    STATE.with_borrow_mut(|s| {
        s.phase = Phase::Idle;
        s.best = None;
        s.requested.clear();
    });
    picker::load_draft(&[]);
    ui.set_wizard_step(if from_welcome { 0 } else { 1 });
    ui.set_wizard_message(SharedString::new());
    ui.set_wizard_open(true);
}

pub fn toggle_pack(ui: &AppWindow, index: i32) {
    STATE.with_borrow_mut(|s| {
        if let Some(v) = usize::try_from(index)
            .ok()
            .and_then(|i| s.selected.get_mut(i))
        {
            *v = !*v;
        }
    });
    refresh_packs(ui);
}

fn refresh_packs(ui: &AppWindow) {
    let packs: Vec<LabPack> = STATE.with_borrow(|s| {
        DOMAIN_PACKS
            .iter()
            .zip(&s.selected)
            .map(|(p, sel)| LabPack {
                name: p.name.into(),
                selected: *sel,
            })
            .collect()
    });
    ui.set_wizard_packs(ModelRc::new(VecModel::from(packs)));
}

pub fn set_where(ui: &AppWindow, index: i32) {
    let w = match index {
        1 if whole_supported() => Where::WholeComputer,
        1 => return,
        2 => Where::Proxy,
        _ if apps_supported() => Where::Apps,
        _ => return,
    };
    STATE.with_borrow_mut(|s| s.where_ = w);
    ui.set_wizard_where(index);
}

fn domains(ui: &AppWindow) -> (Vec<String>, Vec<String>, Vec<&'static str>) {
    let custom = parse_domains(&ui.get_wizard_custom_sites());
    STATE.with_borrow(|s| {
        let packs: Vec<_> = DOMAIN_PACKS
            .iter()
            .zip(&s.selected)
            .filter(|(_, sel)| **sel)
            .map(|(p, _)| p)
            .collect();
        let mut domains: Vec<String> = packs
            .iter()
            .flat_map(|p| p.domains.iter().map(|d| (*d).to_owned()))
            .collect();
        let mut probes: Vec<String> = packs
            .iter()
            .flat_map(|p| p.probes.iter().map(|d| (*d).to_owned()))
            .collect();
        domains.extend(custom.iter().cloned());
        probes.extend(custom);
        (domains, probes, packs.iter().map(|p| p.name).collect())
    })
}

/// Validates the current step and moves forward.
pub fn next(ui: &AppWindow) {
    let step = ui.get_wizard_step();
    match step {
        1 => {
            if domains(ui).0.is_empty() {
                ui.set_wizard_message(tr("Pick at least one site.").into());
                return;
            }
            // Offer the matching desktop app when Discord is chosen.
            if STATE.with_borrow(|s| s.selected.first().copied().unwrap_or(false)) {
                suggest_discord_app();
            }
        }
        2 => {
            let needs_apps = STATE.with_borrow(|s| s.where_ == Where::Apps);
            if needs_apps && picker::draft_keys().is_empty() {
                ui.set_wizard_message(tr("Add at least one app (for example Discord).").into());
                return;
            }
        }
        3 => {
            ui.set_wizard_step(4);
            start(ui);
            return;
        }
        _ => {}
    }
    ui.set_wizard_message(SharedString::new());
    ui.set_wizard_step(step + 1);
}

pub fn back(ui: &AppWindow) {
    STATE.with_borrow_mut(|s| s.phase = Phase::Idle);
    ui.set_wizard_message(SharedString::new());
    let step = ui.get_wizard_step();
    ui.set_wizard_step(if step == 4 { 3 } else { (step - 1).max(1) });
}

fn suggest_discord_app() {
    if picker::draft_keys().is_empty() {
        picker::add_manual_key("Discord");
    }
}

fn engine() -> EngineKind {
    STATE.with_borrow(|s| match s.where_ {
        Where::WholeComputer if Os::current() == Os::Windows => EngineKind::ZapretWinws,
        Where::WholeComputer => EngineKind::ZapretNfqws,
        Where::Proxy => proxy_engine(),
        Where::Apps => EngineKind::ByeDpi,
    })
}

fn required_packages() -> Vec<PackageId> {
    STATE.with_borrow(|s| match s.where_ {
        // Windows routes apps with ProxiFyre and its driver; Linux does it in the service.
        Where::Apps if Os::current() == Os::Windows => vec![
            PackageId::ByeDpi,
            PackageId::PacketFilterDriver,
            PackageId::ProxiFyre,
        ],
        Where::Apps => vec![PackageId::ByeDpi],
        Where::Proxy => vec![PackageId::for_engine(proxy_engine()).unwrap_or(PackageId::ByeDpi)],
        Where::WholeComputer => vec![PackageId::Zapret],
    })
}

/// Step 4: install → find strategy → create profile. Each stage is advanced by an event.
pub fn start(ui: &AppWindow) {
    if !ui.get_service_connected() {
        fail(
            ui,
            &tr(
                "The DPIMech background service is not running. Go back and press “Install the service”.",
            ),
        );
        return;
    }
    let (_, _, names) = domains(ui);
    let taken: Vec<u16> =
        PROFILES.with_borrow(|p| p.iter().filter_map(|s| s.profile.routing.port()).collect());
    let port = (1080..1200).find(|p| !taken.contains(p)).unwrap_or(1080);
    STATE.with_borrow_mut(|s| {
        s.phase = Phase::Installing;
        s.requested.clear();
        s.best = None;
        s.baseline_ok = false;
        s.profile_name = names.join(" + ");
        s.port = port;
        s.lab_done = 0.0;
        s.failed_at = None;
        s.strategy_errors.clear();
    });
    ui.set_wizard_message(SharedString::new());
    ui.set_wizard_busy(true);
    ui.set_wizard_progress(0.0);
    advance(ui);
}

pub fn retry(ui: &AppWindow) {
    start(ui);
}

/// Moves the state machine forward as far as the current information allows.
fn advance(ui: &AppWindow) {
    let phase = STATE.with_borrow(|s| s.phase.clone());
    match phase {
        Phase::Installing => {
            let packages = STATE.with_borrow(|s| s.packages.clone());
            for id in required_packages() {
                let info = packages.iter().find(|p| p.id == id);
                if info.is_some_and(|p| p.installed_version.is_some()) {
                    continue;
                }
                if let Some(PackageTask::Failed { message }) = info.map(|p| &p.task) {
                    fail(
                        ui,
                        &trf!("Installing {} failed: {}", id.display_name(), message),
                    );
                    return;
                }
                let already = STATE.with_borrow(|s| s.requested.contains(&id));
                if !already {
                    STATE.with_borrow_mut(|s| s.requested.push(id));
                    send(Command::InstallPackage(id));
                }
                show_tasks(ui);
                return; // wait for the next package update
            }
            STATE.with_borrow_mut(|s| s.phase = Phase::LoadingStrategies);
            send(Command::LabStrategies(engine()));
            show_tasks(ui);
        }
        _ => show_tasks(ui),
    }
}

pub fn on_packages(ui: &AppWindow, packages: &[PackageInfo]) {
    STATE.with_borrow_mut(|s| s.packages = packages.to_vec());
    if STATE.with_borrow(|s| s.phase == Phase::Installing) {
        advance(ui);
    }
}

pub fn on_strategies(ui: &AppWindow, strategies: &[LabStrategy]) {
    if STATE.with_borrow(|s| s.phase != Phase::LoadingStrategies) {
        return;
    }
    let (domains, probes, _) = domains(ui);
    STATE.with_borrow_mut(|s| s.phase = Phase::Testing);
    send(Command::StartLab(LabRequest {
        engine: engine(),
        strategies: strategies.to_vec(),
        domains,
        probes,
        repeats: 1,
    }));
    show_tasks(ui);
}

pub fn on_lab_progress(ui: &AppWindow, done: u32, total: u32, result: &LabResult) {
    if STATE.with_borrow(|s| s.phase != Phase::Testing) {
        return;
    }
    STATE.with_borrow_mut(|s| s.lab_done = done as f32 / total.max(1) as f32);
    ui.set_wizard_progress(overall_progress());
    STATE.with_borrow_mut(|s| {
        if result.strategy.is_some() {
            s.strategy_errors.push(result.error.clone());
        }
        if result.strategy.is_none() {
            s.baseline_ok = result.total > 0 && result.ok == result.total;
        } else if result.error.is_none()
            && result.ok > 0
            && s.best.as_ref().is_none_or(|b| result.score() > b.score())
        {
            s.best = Some(result.clone());
        }
    });
}

pub fn on_lab_finished(ui: &AppWindow, cancelled: bool, error: Option<String>) {
    if STATE.with_borrow(|s| s.phase != Phase::Testing) {
        return;
    }
    if cancelled {
        fail(ui, &tr("The test was cancelled."));
        return;
    }
    if let Some(e) = error {
        fail(ui, &trf!("The test failed: {}", e));
        return;
    }
    let (best, baseline_ok) = STATE.with_borrow(|s| (s.best.clone(), s.baseline_ok));
    if baseline_ok {
        STATE.with_borrow_mut(|s| s.phase = Phase::Done);
        finish_with(
            ui,
            &tr(
                "Good news: these sites already open normally on your connection, so nothing needs to be changed.",
            ),
        );
        return;
    }
    let Some(best) = best else {
        fail(ui, &no_setting_message());
        return;
    };
    save_profile(ui, &best);
}

pub fn on_lab_error(ui: &AppWindow, message: &str) {
    if STATE.with_borrow(|s| s.phase == Phase::Testing) {
        fail(ui, message);
    }
}

fn save_profile(ui: &AppWindow, best: &LabResult) {
    let Some(strategy) = &best.strategy else {
        return;
    };
    let (domains, _, _) = domains(ui);
    let (where_, name, port) = STATE.with_borrow(|s| (s.where_, s.profile_name.clone(), s.port));
    let routing = match where_ {
        Where::Apps => Routing::PerApp {
            apps: picker::draft_keys(),
            port,
            tcp_only: false,
        },
        Where::Proxy => Routing::LocalProxy { port },
        Where::WholeComputer => Routing::SystemWide {
            domains: domains.clone(),
        },
    };
    let profile = Profile {
        id: Profile::new_id(),
        name: if name.is_empty() {
            tr("My sites")
        } else {
            name
        },
        engine: engine(),
        args: strategy.args.clone(),
        routing,
        autostart: true,
        reliability: Reliability::default(),
        check_sites: crate::convert::check_sites_for(&domains),
    };
    STATE.with_borrow_mut(|s| s.phase = Phase::Saving);
    show_tasks(ui);
    send(Command::SaveAndStart(profile));
}

pub fn on_profile_saved(ui: &AppWindow, result: Result<(), String>) {
    if STATE.with_borrow(|s| s.phase != Phase::Saving) {
        return;
    }
    if let Err(e) = result {
        fail(ui, &trf!("Could not create the profile: {}", e));
        return;
    }
    let (best, where_, name, port) =
        STATE.with_borrow(|s| (s.best.clone(), s.where_, s.profile_name.clone(), s.port));
    STATE.with_borrow_mut(|s| s.phase = Phase::Done);
    let mut text = trf!(
        "“{}” is on and starts automatically with the computer.",
        name
    );
    if let Some(best) = best {
        text.push(' ');
        text.push_str(&trf!(
            "{}/{} test requests worked ({} ms on average).",
            best.ok,
            best.total,
            best.avg_ms
        ));
    }
    match where_ {
        Where::Proxy => {
            text.push(' ');
            text.push_str(&trf!(
                "Set your browser or app to use the SOCKS5 proxy 127.0.0.1:{}.",
                port
            ));
        }
        Where::Apps => {
            text.push(' ');
            text.push_str(&tr("Restart the chosen apps if they were already open."));
        }
        Where::WholeComputer => {}
    }
    finish_with(ui, &text);
}

fn finish_with(ui: &AppWindow, text: &str) {
    ui.set_wizard_busy(false);
    ui.set_wizard_progress(1.0);
    ui.set_wizard_message(text.into());
    ui.set_wizard_step(5);
}

/// Why nothing worked, in the most useful words available.
fn no_setting_message() -> String {
    let (errors, where_) = STATE.with_borrow(|s| (s.strategy_errors.clone(), s.where_));
    // Every strategy failed to even start for the same reason (e.g. missing kernel support):
    // the strategies are not the problem, so say what is. Details after the first ": " may
    // differ per strategy (ports, rule text), the headline does not.
    let headline = |e: &str| e.split(": ").next().unwrap_or(e).trim().to_owned();
    if let Some(Some(first)) = errors.first()
        && errors
            .iter()
            .all(|e| e.as_deref().map(headline) == Some(headline(first)))
    {
        return trf!(
            "The engine could not run on this computer: {}.",
            headline(first)
        );
    }
    let hint = match where_ {
        Where::WholeComputer => tr(
            "Try again later, add other sites, or run the Strategy Lab from the full interface to test more settings.",
        ),
        _ => {
            tr("Try “The whole computer” (zapret) or run the Strategy Lab from the full interface.")
        }
    };
    format!(
        "{} {hint}",
        tr("No setting worked on your connection for these sites.")
    )
}

fn fail(ui: &AppWindow, text: &str) {
    STATE.with_borrow_mut(|s| {
        s.failed_at = Some(s.phase.clone());
        s.phase = Phase::Failed;
    });
    ui.set_wizard_busy(false);
    ui.set_wizard_message(text.into());
    show_tasks(ui);
}

/// One bar for the whole run: installing is ~20 %, the Lab ~70 %, saving the rest. A package
/// being downloaded counts with its download percentage.
fn overall_progress() -> f32 {
    STATE.with_borrow(|s| {
        let required = required_packages();
        let installs: f32 = required
            .iter()
            .map(|id| match s.packages.iter().find(|p| p.id == *id) {
                Some(p) if p.installed_version.is_some() => 1.0,
                Some(p) => match p.task {
                    PackageTask::Downloading { percent } => f32::from(percent) / 100.0 * 0.9,
                    PackageTask::Installing => 0.95,
                    _ => 0.0,
                },
                None => 0.0,
            })
            .sum::<f32>()
            / required.len().max(1) as f32;
        match s.phase {
            Phase::Installing => 0.2 * installs,
            Phase::LoadingStrategies => 0.2,
            Phase::Testing => 0.2 + 0.7 * s.lab_done,
            Phase::Saving => 0.95,
            Phase::Done => 1.0,
            Phase::Idle | Phase::Failed => 0.0,
        }
    })
}

fn show_tasks(ui: &AppWindow) {
    if STATE.with_borrow(|s| s.phase != Phase::Failed) {
        ui.set_wizard_progress(overall_progress());
    }
    let (phase, packages, requested, failed_at) = STATE.with_borrow(|s| {
        (
            s.phase.clone(),
            s.packages.clone(),
            s.requested.clone(),
            s.failed_at.clone(),
        )
    });
    let mut tasks = Vec::new();
    for id in required_packages() {
        let info = packages.iter().find(|p| p.id == id);
        let state = match info.map(|p| (&p.task, p.installed_version.is_some())) {
            Some((_, true)) => "done",
            Some((PackageTask::Failed { .. }, _)) => "failed",
            Some((PackageTask::Downloading { .. } | PackageTask::Installing, _)) => "running",
            _ if requested.contains(&id) => "running",
            _ => "pending",
        };
        tasks.push(WizardTask {
            text: trf!("Install {}", id.display_name()).into(),
            state: state.into(),
        });
    }
    let stage = |active: Phase, after: &[Phase]| -> &'static str {
        if phase == Phase::Failed && failed_at.as_ref() == Some(&active) {
            "failed"
        } else if phase == active {
            "running"
        } else if after.contains(&phase) {
            "done"
        } else {
            "pending"
        }
    };
    tasks.push(WizardTask {
        text: tr("Find a setting that works on your connection").into(),
        state: match (&phase, &failed_at) {
            (Phase::Failed, Some(Phase::LoadingStrategies)) => "failed",
            (Phase::LoadingStrategies, _) => "running",
            _ => stage(Phase::Testing, &[Phase::Saving, Phase::Done]),
        }
        .into(),
    });
    tasks.push(WizardTask {
        text: tr("Create the profile and turn it on").into(),
        state: stage(Phase::Saving, &[Phase::Done]).into(),
    });
    ui.set_wizard_tasks(ModelRc::new(VecModel::from(tasks)));
}

pub fn close(ui: &AppWindow) {
    STATE.with_borrow_mut(|s| s.phase = Phase::Idle);
    ui.set_wizard_open(false);
    let _ = ui.window().show();
}
