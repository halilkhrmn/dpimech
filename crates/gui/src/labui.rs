//! Strategy Lab page state (UI thread only).

use crate::i18n::tr;
use std::cell::RefCell;

use dpimech_core::catalog::{DOMAIN_PACKS, STANDARD_SET};
use dpimech_core::lab::{IspInfo, LabRequest, LabResult, LabStrategy};
use dpimech_core::model::{EngineKind, Os};
use slint::{ModelRc, SharedString, VecModel};

use crate::convert::parse_domains;
use crate::{AppWindow, LabPack, LabRow};

#[derive(Default)]
struct State {
    engines: Vec<EngineKind>,
    selected_packs: Vec<bool>,
    strategies: Vec<LabStrategy>,
    results: Vec<LabResult>,
    /// Domains of the running/last test, reused when a result becomes a profile.
    domains: Vec<String>,
    engine: Option<EngineKind>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Engines that have strategies to test on this OS.
pub fn lab_engines() -> Vec<EngineKind> {
    EngineKind::available_on(Os::current())
        .filter(|e| !dpimech_core::catalog::builtin_strategies(*e).is_empty())
        .collect()
}

pub fn init(ui: &AppWindow) {
    let engines = lab_engines();
    ui.set_lab_engines(ModelRc::new(VecModel::from(
        engines
            .iter()
            .map(|e| SharedString::from(e.display_name()))
            .collect::<Vec<_>>(),
    )));
    STATE.with_borrow_mut(|s| {
        s.engines = engines;
        // Discord is what most people come for.
        s.selected_packs = DOMAIN_PACKS.iter().map(|p| p.id == "discord").collect();
    });
    refresh_packs(ui);
    ui.set_lab_isp_text(tr("ISP not detected").into());
    ui.set_lab_status(tr("Pick sites and press Start.").into());
}

pub fn engine_at(index: i32) -> Option<EngineKind> {
    STATE.with_borrow(|s| {
        usize::try_from(index)
            .ok()
            .and_then(|i| s.engines.get(i).copied())
    })
}

pub fn toggle_pack(ui: &AppWindow, index: i32) {
    STATE.with_borrow_mut(|s| {
        if let Some(v) = usize::try_from(index)
            .ok()
            .and_then(|i| s.selected_packs.get_mut(i))
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
            .zip(&s.selected_packs)
            .map(|(p, sel)| LabPack {
                name: p.name.into(),
                selected: *sel,
            })
            .collect()
    });
    ui.set_lab_packs(ModelRc::new(VecModel::from(packs)));
}

pub fn set_strategies(ui: &AppWindow, strategies: Vec<LabStrategy>) {
    let standard = strategies
        .iter()
        .filter(|s| s.source == STANDARD_SET)
        .count();
    let recommended = strategies.iter().filter(|s| s.recommended).count();
    let mut summary = trf!(
        "{} to try: {} from the DPIMech standard set",
        strategies.len(),
        standard
    );
    // One entry per online list, e.g. "60 from Community list".
    let mut online: Vec<(String, String, usize)> = Vec::new();
    for s in strategies.iter().filter(|s| s.source != STANDARD_SET) {
        match online.iter_mut().find(|(label, _, _)| *label == s.source) {
            Some(entry) => entry.2 += 1,
            None => online.push((s.source.clone(), s.origin.clone(), 1)),
        }
    }
    for (label, _, count) in &online {
        summary.push_str(", ");
        summary.push_str(&trf!("{} from {}", count, label));
    }
    if online.is_empty() {
        summary.push_str(" — ");
        summary.push_str(&tr("“Update online lists” adds more"));
    }
    if recommended > 0 {
        summary.push_str(" · ");
        summary.push_str(&trf!("★ {} made for your ISP", recommended));
    }
    ui.set_lab_strategy_summary(summary.into());

    let mut about = tr("Standard set: well-known settings that ship with DPIMech.");
    for (label, origin, _) in &online {
        about.push(' ');
        about.push_str(&trf!("{}: downloaded from {}.", label, origin));
    }
    ui.set_lab_sources_note(about.into());
    STATE.with_borrow_mut(|s| s.strategies = strategies);
}

pub fn set_isp(ui: &AppWindow, info: &IspInfo) {
    let name = info.known.clone().unwrap_or_else(|| info.provider.clone());
    let asn = info.asn.map(|a| format!(" · AS{a}")).unwrap_or_default();
    let text: SharedString = format!("{name}{asn} · {}", info.country).into();
    ui.set_lab_isp_text(text.clone());
    ui.set_wizard_isp_text(text);
}

/// Builds the test request, or explains what is missing.
pub fn request(ui: &AppWindow) -> Result<LabRequest, String> {
    let engine = engine_at(ui.get_lab_engine_index()).ok_or("Choose an engine.")?;
    let mut domains: Vec<String> = STATE.with_borrow(|s| {
        DOMAIN_PACKS
            .iter()
            .zip(&s.selected_packs)
            .filter(|(_, sel)| **sel)
            .flat_map(|(p, _)| p.domains.iter().map(|d| (*d).to_owned()))
            .collect()
    });
    for d in parse_domains(&ui.get_lab_custom_domains()) {
        if !domains.contains(&d) {
            domains.push(d);
        }
    }
    if domains.is_empty() {
        return Err("Pick at least one site.".into());
    }
    let custom = parse_domains(&ui.get_lab_custom_domains());
    let probes: Vec<String> = STATE.with_borrow(|s| {
        DOMAIN_PACKS
            .iter()
            .zip(&s.selected_packs)
            .filter(|(_, sel)| **sel)
            .flat_map(|(p, _)| p.probes.iter().map(|d| (*d).to_owned()))
            .chain(custom)
            .collect()
    });
    let strategies = STATE.with_borrow(|s| s.strategies.clone());
    if strategies.is_empty() {
        return Err("No strategies loaded yet.".into());
    }
    Ok(LabRequest {
        engine,
        strategies,
        domains,
        probes,
        repeats: ui.get_lab_repeats().clamp(1, 5) as u8,
    })
}

pub fn started(ui: &AppWindow, request: &LabRequest) {
    STATE.with_borrow_mut(|s| {
        s.results.clear();
        s.domains = request.domains.clone();
        s.engine = Some(request.engine);
    });
    ui.set_lab_results(ModelRc::new(VecModel::<LabRow>::default()));
    ui.set_lab_baseline(SharedString::new());
    ui.set_lab_progress(0.0);
    ui.set_lab_running(true);
    ui.set_lab_status(
        trf!(
            "Testing {} strategies… engines run briefly in the background.",
            request.strategies.len()
        )
        .into(),
    );
}

pub fn progress(ui: &AppWindow, done: u32, total: u32, result: LabResult) {
    ui.set_lab_progress(done as f32 / total.max(1) as f32);
    ui.set_lab_status(format!("{done} / {total}").into());
    if result.strategy.is_none() {
        let text = if result.ok == result.total {
            tr(
                "Without any bypass every test site already works — nothing to fix on this network for these sites.",
            )
        } else {
            trf!(
                "Without any bypass only {}/{} requests succeed. Blocked: {}.",
                result.ok,
                result.total,
                result.failed_domains.join(", ")
            )
        };
        ui.set_lab_baseline(text.into());
        return;
    }
    let rows: Vec<LabRow> = STATE.with_borrow_mut(|s| {
        s.results.push(result);
        s.results.sort_by(|a, b| {
            b.error
                .is_none()
                .cmp(&a.error.is_none())
                .then(b.score().cmp(&a.score()))
        });
        s.results.iter().map(row).collect()
    });
    ui.set_lab_results(ModelRc::new(VecModel::from(rows)));
}

pub fn finished(ui: &AppWindow, cancelled: bool, error: Option<String>) {
    ui.set_lab_running(false);
    let working = STATE.with_borrow(|s| {
        s.results
            .iter()
            .filter(|r| r.error.is_none() && r.total > 0 && r.ok == r.total)
            .count()
    });
    let status = match (cancelled, error) {
        (true, _) => tr("Cancelled."),
        (_, Some(e)) => trf!("Stopped: {}", e),
        _ if working > 0 => trf!(
            "Done — {} strategies work on every site. Press “Use” on the top one.",
            working
        ),
        _ => tr(
            "Done — none worked on every site. Try more tries per site, another engine, or updated online lists.",
        ),
    };
    ui.set_lab_progress(1.0);
    ui.set_lab_status(status.into());
}

/// The chosen result as (engine, args, domains) for a new profile.
pub fn result_at(index: i32) -> Option<(EngineKind, String, String, Vec<String>)> {
    STATE.with_borrow(|s| {
        let r = s.results.get(usize::try_from(index).ok()?)?;
        let strategy = r.strategy.as_ref()?;
        Some((
            s.engine?,
            strategy.name.clone(),
            strategy.args.clone(),
            s.domains.clone(),
        ))
    })
}

fn row(r: &LabResult) -> LabRow {
    let (name, source, args, recommended) = match &r.strategy {
        Some(s) => (
            s.name.clone(),
            s.source.clone(),
            s.args.clone(),
            s.recommended,
        ),
        None => (tr("No bypass"), String::new(), String::new(), false),
    };
    LabRow {
        name: name.into(),
        source: source.into(),
        args: args.into(),
        ok: r.ok as i32,
        total: r.total as i32,
        ms: r.avg_ms as i32,
        error: r.error.clone().unwrap_or_default().into(),
        recommended,
    }
}
