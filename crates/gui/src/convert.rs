//! Conversions between core model types and Slint view structs.

use crate::i18n::tr;
use std::time::{SystemTime, UNIX_EPOCH};

use dpimech_core::model::{
    DEFAULT_PROXY_PORT, EngineKind, LogLevel, LogLine, Os, Profile, ProfileState, ProfileStatus,
    Reliability, Routing, RoutingMode, SlowAction,
};
use dpimech_core::packages::{PackageInfo, PackageKind, PackageTask};
use slint::Image;

use crate::{AppItem, LogItem, PackageItem, ProfileDraft, ProfileItem};

pub fn modes_for(engines: &[EngineKind], engine_index: i32) -> Vec<RoutingMode> {
    usize::try_from(engine_index)
        .ok()
        .and_then(|i| engines.get(i))
        .map(|e| e.routing_modes(Os::current()))
        .unwrap_or_default()
}

pub fn empty_draft() -> ProfileDraft {
    ProfileDraft {
        id: "".into(),
        name: "".into(),
        engine_index: 0,
        routing_index: 0,
        port: DEFAULT_PROXY_PORT as i32,
        args: "".into(),
        autostart: false,
        auto_restart: true,
        restart_hours: 0,
        tcp_only: false,
        domains: "".into(),
        health: "".into(),
        check_index: check_index(Reliability::default().check_every_minutes),
        slow_warn_only: false,
        check_sites: "".into(),
    }
}

/// Choices offered for the connection check interval, in minutes (0 = off).
pub const CHECK_MINUTES: [u32; 5] = [0, 1, 3, 5, 10];

fn check_index(minutes: u32) -> i32 {
    CHECK_MINUTES
        .iter()
        .position(|m| *m == minutes)
        .unwrap_or(3) as i32
}

/// A few reachable hosts to monitor for these domains: the probe hosts of every domain pack
/// the domains come from (at most three per pack), plus custom domains as they are.
pub fn check_sites_for(domains: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let packs = dpimech_core::catalog::domain_packs();
    for pack in packs.iter() {
        if pack.domains.iter().any(|d| domains.contains(d)) {
            out.extend(pack.probes.iter().take(3).cloned());
        }
    }
    for d in domains {
        let in_pack = packs.iter().any(|p| p.domains.contains(d));
        if !in_pack && !out.contains(d) {
            out.push(d.clone());
        }
    }
    out
}

/// One domain per line (commas and spaces also accepted); duplicates removed.
pub fn parse_domains(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for d in text.split(|c: char| c == ',' || c.is_whitespace()) {
        let d = d
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_ascii_lowercase();
        if !d.is_empty() && !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

/// Appends a domain pack to the editor's text, skipping domains already listed.
pub fn add_domains(text: &str, extra: &[String]) -> String {
    let mut all = parse_domains(text);
    for d in extra {
        if !all.contains(d) {
            all.push(d.clone());
        }
    }
    all.join("\n")
}

/// Returns the editable draft plus the profile's per-app list.
pub fn draft_from_profile(p: &Profile, engines: &[EngineKind]) -> (ProfileDraft, Vec<String>) {
    let engine_index = engines.iter().position(|e| *e == p.engine).unwrap_or(0) as i32;
    let routing_index = modes_for(engines, engine_index)
        .iter()
        .position(|m| *m == p.routing.mode())
        .unwrap_or(0) as i32;
    let (apps, tcp_only) = match &p.routing {
        Routing::PerApp { apps, tcp_only, .. } => (apps.clone(), *tcp_only),
        _ => (Vec::new(), false),
    };
    let draft = ProfileDraft {
        id: p.id.as_str().into(),
        name: p.name.as_str().into(),
        engine_index,
        routing_index,
        port: p.routing.port().unwrap_or(DEFAULT_PROXY_PORT) as i32,
        args: p.args.as_str().into(),
        autostart: p.autostart,
        auto_restart: p.reliability.auto_restart,
        restart_hours: p.reliability.restart_every_hours as i32,
        check_index: check_index(p.reliability.check_every_minutes),
        slow_warn_only: p.reliability.on_slow == SlowAction::Warn,
        check_sites: p.check_sites.join(", ").into(),
        tcp_only,
        health: "".into(),
        domains: match &p.routing {
            Routing::SystemWide { domains } => domains.join("\n").into(),
            _ => "".into(),
        },
    };
    (draft, apps)
}

pub fn profile_from_draft(
    d: &ProfileDraft,
    engines: &[EngineKind],
    apps: Vec<String>,
) -> Result<Profile, String> {
    let name = d.name.trim();
    if name.is_empty() {
        return Err(tr("Give the profile a name."));
    }
    let engine = *usize::try_from(d.engine_index)
        .ok()
        .and_then(|i| engines.get(i))
        .ok_or_else(|| tr("Choose an engine."))?;
    let mode = usize::try_from(d.routing_index)
        .ok()
        .and_then(|i| modes_for(engines, d.engine_index).get(i).copied())
        .ok_or_else(|| tr("Choose a routing mode."))?;
    let port = u16::try_from(d.port)
        .ok()
        .filter(|p| *p >= 1024)
        .ok_or_else(|| tr("Port must be between 1024 and 65535."))?;
    let routing = match mode {
        RoutingMode::SystemWide => Routing::SystemWide {
            domains: parse_domains(&d.domains),
        },
        RoutingMode::LocalProxy => Routing::LocalProxy { port },
        RoutingMode::PerApp => {
            if apps.is_empty() {
                return Err(tr("Add at least one application for per-app routing."));
            }
            Routing::PerApp {
                apps,
                port,
                tcp_only: d.tcp_only,
            }
        }
    };
    Ok(Profile {
        id: d.id.to_string(),
        name: name.to_owned(),
        engine,
        args: d.args.trim().to_owned(),
        routing,
        autostart: d.autostart,
        reliability: Reliability {
            auto_restart: d.auto_restart,
            restart_every_hours: d.restart_hours.clamp(0, 168) as u32,
            check_every_minutes: usize::try_from(d.check_index)
                .ok()
                .and_then(|i| CHECK_MINUTES.get(i).copied())
                .unwrap_or(5),
            on_slow: if d.slow_warn_only {
                SlowAction::Warn
            } else {
                SlowAction::Restart
            },
        },
        check_sites: parse_domains(&d.check_sites),
    })
}

pub fn profile_item(s: &ProfileState) -> ProfileItem {
    let p = &s.profile;
    let detail = match &p.routing {
        Routing::PerApp {
            apps,
            port,
            tcp_only,
        } => {
            let text = trf!("{} · port {}", apps.join(", "), port);
            if *tcp_only {
                format!("{text} · {}", tr("TCP only"))
            } else {
                text
            }
        }
        Routing::LocalProxy { port } => format!("127.0.0.1:{port}"),
        Routing::SystemWide { domains } => match domains.len() {
            0 => tr("All matching traffic"),
            n => trf!(
                "{} site(s): {}",
                n,
                domains
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
    };
    let (status, status_text) = match &s.status {
        ProfileStatus::Stopped => ("stopped", tr("Stopped")),
        ProfileStatus::Starting => ("starting", tr("Starting…")),
        ProfileStatus::Running {
            since_unix,
            restarts,
            health,
        } => {
            let restarts = match restarts {
                0 => String::new(),
                1 => format!(" · {}", tr("1 auto-restart")),
                n => format!(" · {}", trf!("{} auto-restarts", n)),
            };
            match health {
                Some(h) if h.advice.is_some() => {
                    ("error", tr(h.advice.as_deref().unwrap_or_default()))
                }
                Some(h) if h.slow => (
                    "starting",
                    if h.ok == 0 {
                        trf!("Sites not answering ({}/{})", h.ok, h.total)
                    } else if h.usual_ms > 0 {
                        trf!("Slow: avg. {} ms (usually {} ms)", h.latency_ms, h.usual_ms)
                    } else {
                        trf!("Only {}/{} sites answer", h.ok, h.total)
                    },
                ),
                Some(h) => (
                    "running",
                    format!(
                        "{}{restarts} · {}",
                        trf!("Running · {}", uptime(*since_unix)),
                        trf!("avg. {} ms", h.latency_ms)
                    ),
                ),
                None => (
                    "running",
                    format!("{}{restarts}", trf!("Running · {}", uptime(*since_unix))),
                ),
            }
        }
        ProfileStatus::Error { message } => ("error", message.clone()),
    };
    ProfileItem {
        id: p.id.as_str().into(),
        name: p.name.as_str().into(),
        engine: p.engine.display_name().into(),
        routing: tr(p.routing.mode().display_name()).into(),
        detail: detail.into(),
        status: status.into(),
        status_text: status_text.into(),
        on: s.status.is_active(),
        can_check: s.status.is_active() && !p.monitored_sites().is_empty(),
    }
}

pub fn package_item(p: &PackageInfo) -> PackageItem {
    let (state, percent, message) = match &p.task {
        PackageTask::Idle => ("idle", 0, String::new()),
        PackageTask::Downloading { percent } => ("downloading", *percent as i32, String::new()),
        PackageTask::Installing => ("installing", 0, String::new()),
        PackageTask::Failed { message } => ("failed", 0, message.clone()),
    };
    PackageItem {
        id: p.id.slug().into(),
        name: p.id.display_name().into(),
        description: p.id.description().into(),
        installed: p.installed_version.clone().unwrap_or_default().into(),
        latest: p.latest_version.clone().unwrap_or_default().into(),
        published: p.latest_published.clone().unwrap_or_default().into(),
        notes: p
            .latest_notes
            .as_deref()
            .map(trim_notes)
            .unwrap_or_default()
            .into(),
        state: state.into(),
        percent,
        message: message.into(),
        update_available: p.update_available(),
        removable: p.id.kind() == PackageKind::Archive,
    }
}

/// Release notes can be long; the card only needs the first part.
fn trim_notes(notes: &str) -> String {
    const MAX: usize = 1200;
    let notes = notes.trim();
    match notes.char_indices().nth(MAX) {
        Some((cut, _)) => format!("{}…", &notes[..cut]),
        None => notes.to_owned(),
    }
}

pub fn app_item(title: &str, exe: &str, path: &str, running: bool, icon: Option<Image>) -> AppItem {
    AppItem {
        title: title.into(),
        exe: exe.into(),
        path: path.into(),
        running,
        has_icon: icon.is_some(),
        icon: icon.unwrap_or_default(),
        letter: title
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default()
            .into(),
    }
}

pub fn log_item(line: &LogLine) -> LogItem {
    let time = chrono::DateTime::from_timestamp_millis(line.unix_ms as i64)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%H:%M:%S")
                .to_string()
        })
        .unwrap_or_default();
    LogItem {
        time: time.into(),
        source: line.source.as_str().into(),
        text: line.text.as_str().into(),
        level: match line.level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
        .into(),
    }
}

fn uptime(since_unix: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(since_unix);
    let s = now.saturating_sub(since_unix);
    match s {
        0..60 => tr("just now"),
        60..3600 => trf!("{} min", s / 60),
        _ => trf!("{} h {} min", s / 3600, (s / 60) % 60),
    }
}

/// "Last check: 8/8 sites · avg. 235 ms (usually 230 ms)" for the profile editor.
pub fn health_text(status: &ProfileStatus) -> String {
    let ProfileStatus::Running {
        health: Some(h), ..
    } = status
    else {
        return String::new();
    };
    let mut text = trf!("Last check: {}/{} sites answered", h.ok, h.total);
    if h.ok > 0 {
        text.push_str(" · ");
        text.push_str(&trf!("avg. {} ms", h.latency_ms));
        if h.usual_ms > 0 {
            text.push(' ');
            text.push_str(&trf!("(usually {} ms)", h.usual_ms));
        }
    }
    if let Some(advice) = &h.advice {
        text.push_str(" — ");
        text.push_str(&tr(advice));
    }
    text
}

/// Where a row of site chips is shown; each has its own font and padding.
#[derive(Clone, Copy)]
pub enum Chips {
    Lab = 0,
    Wizard = 1,
    Editor = 2,
}

thread_local! {
    /// Width of each chip area as Slint last reported it.
    static CHIP_WIDTHS: std::cell::Cell<[f32; 3]> = const { std::cell::Cell::new([560.0; 3]) };
}

impl Chips {
    /// Rough width of a chip: px per character of its label, and padding + border + "✓ " + spacing.
    fn metrics(self) -> (f32, f32) {
        match self {
            Chips::Lab => (7.5, 48.0),
            Chips::Wizard => (8.2, 60.0),
            Chips::Editor => (7.5, 46.0),
        }
    }

    /// Stores the reported width; true when the rows should be rebuilt.
    pub fn set_width(self, width: f32) -> bool {
        if width <= 0.0 {
            return false; // not laid out yet
        }
        CHIP_WIDTHS.with(|w| {
            let mut all = w.get();
            let changed = (all[self as usize] - width).abs() >= 1.0;
            all[self as usize] = width;
            w.set(all);
            changed
        })
    }
}

/// The user's country, read once (sites blocked there are offered first).
pub fn country() -> &'static str {
    static COUNTRY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    COUNTRY.get_or_init(crate::i18n::system_country)
}

/// The site packs as chips, the user's country first, split into rows that fit the area
/// (Slint has no wrapping layout).
pub fn pack_rows(place: Chips, selected: &[String]) -> slint::ModelRc<crate::PackRow> {
    let packs = dpimech_core::catalog::domain_packs();
    let width = CHIP_WIDTHS.with(|w| w.get()[place as usize]);
    let (per_char, extra) = place.metrics();
    let lang = crate::i18n::current();
    let mut rows: Vec<Vec<crate::LabPack>> = Vec::new();
    let mut used = f32::MAX;
    for i in dpimech_core::catalog::pack_order(&packs, country()) {
        let pack = &packs[i];
        let name = pack.display_name(lang);
        let chip = name.chars().count() as f32 * per_char + extra;
        if used + chip > width {
            rows.push(Vec::new());
            used = 0.0;
        }
        used += chip;
        rows.last_mut().unwrap().push(crate::LabPack {
            name: name.into(),
            selected: selected.contains(&pack.id),
            index: i as i32,
        });
    }
    slint::ModelRc::new(slint::VecModel::from(
        rows.into_iter()
            .map(|packs| crate::PackRow {
                packs: slint::ModelRc::new(slint::VecModel::from(packs)),
            })
            .collect::<Vec<_>>(),
    ))
}
