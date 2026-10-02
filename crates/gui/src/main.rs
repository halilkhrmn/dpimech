#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod i18n;
mod apps;
mod autostart;
mod bridge;
mod convert;
mod hotkey;
mod labui;
mod launcher;
mod logfile;
mod notify;
mod picker;
mod prefs;
mod report;
mod selfupdate;
mod shortcut;
mod shortcutui;
mod single;
mod state;
mod tray;
mod wizard;

use crate::i18n::tr;
use dpimech_core::model::{EngineKind, Os, ProfileState, RoutingMode};
use dpimech_core::packages::PackageId;
use slint::{CloseRequestResponse, ComponentHandle, Model, ModelRc, SharedString, VecModel};
use tokio::sync::mpsc;

use crate::bridge::Command;
use crate::state::{DRAFT_APPS, PROFILES};

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    // A profile shortcut: a small progress window, no tray, no single-instance lock.
    let args: Vec<String> = std::env::args().collect();
    if let Some((profile, open)) = launcher::from_args(&args) {
        return launcher::run(profile, open);
    }
    let ui = AppWindow::new()?;
    let weak = ui.as_weak();
    if !single::acquire(move || {
        let _ = weak.upgrade_in_event_loop(|ui| {
            let _ = ui.show();
        });
    }) {
        return Ok(()); // the running instance shows its window instead
    }
    // 0.1.x ran as "dpimngr": keep the user's choices across the rename.
    prefs::migrate_legacy();
    autostart::migrate_legacy();
    notify::register_identity();
    #[cfg(debug_assertions)]
    if std::env::var_os("DPIMECH_DEBUG_TOAST").is_some() {
        notify::show_test();
    }
    let os = Os::current();
    let engines: Vec<EngineKind> = EngineKind::available_on(os).collect();

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    ui.set_os_name(os.display_name().into());
    ui.set_app_version(dpimech_core::VERSION.into());
    ui.on_check_app_update({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || {
            weak.unwrap().set_app_update_text(tr("Checking…").into());
            let _ = tx.send(Command::CheckAppUpdate);
        }
    });
    ui.on_restart_and_update({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || selfupdate::restart_and_update(&weak.unwrap(), &tx)
    });
    ui.on_download_app_update(|| {
        let url = APP_UPDATE_URL.with_borrow(|u| u.clone());
        if let Some(url) = url {
            open_url(&url);
        }
    });
    ui.on_add_defender_exclusion({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || {
            weak.unwrap().set_defender_state("working".into());
            let _ = tx.send(Command::AddDefenderExclusion);
        }
    });
    let startup = autostart::get();
    ui.set_start_with_windows(startup.enabled);
    ui.set_start_minimized(startup.minimized);
    ui.on_set_autostart({
        let weak = ui.as_weak();
        move |enabled, minimized| {
            let ui = weak.unwrap();
            let wanted = autostart::Autostart { enabled, minimized };
            if let Err(e) = autostart::set(wanted) {
                eprintln!("autostart: {e}");
            }
            // Re-read so the switches always show what is really in the registry.
            let actual = autostart::get();
            ui.set_start_with_windows(actual.enabled);
            ui.set_start_minimized(actual.minimized);
        }
    });
    ui.set_engine_names(string_model(engines.iter().map(|e| e.display_name())));
    ui.set_logs(ModelRc::new(VecModel::<LogItem>::default()));
    ui.set_draft_apps(DRAFT_APPS.with(|m| ModelRc::from(m.clone())));
    set_domain_pack_names(&ui);
    ui.on_add_domain_pack({
        let weak = ui.as_weak();
        move |index| {
            let ui = weak.unwrap();
            let Some(pack) = usize::try_from(index)
                .ok()
                .and_then(|i| dpimech_core::catalog::domain_packs().get(i).cloned())
            else {
                return;
            };
            let mut draft = ui.get_draft();
            draft.domains = convert::add_domains(&draft.domains, &pack.domains).into();
            ui.set_draft(draft);
        }
    });

    bridge::spawn(ui.as_weak(), cmd_rx);

    let set_routing_names = {
        let engines = engines.clone();
        move |ui: &AppWindow, engine_index: i32| {
            let modes = convert::modes_for(&engines, engine_index);
            let index_of =
                |m: RoutingMode| modes.iter().position(|x| *x == m).map_or(-1, |i| i as i32);
            ui.set_per_app_index(index_of(RoutingMode::PerApp));
            ui.set_system_wide_index(index_of(RoutingMode::SystemWide));
            let names: Vec<String> = modes.iter().map(|m| tr(m.display_name())).collect();
            ui.set_routing_names(string_model(names.iter().map(String::as_str)));
        }
    };

    ui.on_new_profile({
        let weak = ui.as_weak();
        let set_routing_names = set_routing_names.clone();
        move || {
            let ui = weak.unwrap();
            ui.set_draft(convert::empty_draft());
            picker::load_draft(&[]);
            set_routing_names(&ui, 0);
            ui.set_editor_error(SharedString::new());
            ui.set_page(Page::Editor);
        }
    });

    ui.on_open_profile({
        let weak = ui.as_weak();
        let engines = engines.clone();
        let set_routing_names = set_routing_names.clone();
        move |id| {
            let ui = weak.unwrap();
            let Some(profile) =
                PROFILES.with_borrow(|p| p.iter().find(|s| s.profile.id == id.as_str()).cloned())
            else {
                return;
            };
            let (mut draft, apps) = convert::draft_from_profile(&profile.profile, &engines);
            draft.health = convert::health_text(&profile.status).into();
            set_routing_names(&ui, draft.engine_index);
            picker::load_draft(&apps);
            ui.set_draft(draft);
            ui.set_editor_error(SharedString::new());
            ui.set_page(Page::Editor);
        }
    });

    ui.on_engine_changed({
        let weak = ui.as_weak();
        let set_routing_names = set_routing_names.clone();
        move |index| set_routing_names(&weak.unwrap(), index)
    });

    ui.on_save_profile({
        let weak = ui.as_weak();
        let engines = engines.clone();
        let tx = cmd_tx.clone();
        move |draft| match convert::profile_from_draft(&draft, &engines, picker::draft_keys()) {
            Ok(profile) => {
                let _ = tx.send(Command::Save(profile));
            }
            Err(message) => weak.unwrap().set_editor_error(message.into()),
        }
    });

    ui.on_delete_profile({
        let tx = cmd_tx.clone();
        let weak = ui.as_weak();
        move |id| {
            let id = id.to_string();
            let _ = tx.send(Command::Delete(id.clone()));
            // A shortcut to a deleted profile would only show an error.
            shortcutui::remove_for_profile(weak.clone(), id);
        }
    });

    ui.on_toggle_profile({
        let tx = cmd_tx.clone();
        move |id, on| {
            let id = id.to_string();
            let _ = tx.send(if on {
                Command::Start(id)
            } else {
                Command::Stop(id)
            });
        }
    });

    ui.on_check_profile({
        let tx = cmd_tx.clone();
        move |id| {
            let _ = tx.send(Command::CheckProfile(id.into()));
        }
    });

    ui.on_open_picker({
        let weak = ui.as_weak();
        move || picker::open(&weak.unwrap())
    });
    ui.on_picker_search({
        let weak = ui.as_weak();
        move |q| picker::search(&weak.unwrap(), &q)
    });
    ui.on_picker_pick({
        let weak = ui.as_weak();
        move |app| picker::pick(&weak.unwrap(), app)
    });
    ui.on_picker_add_manual({
        let weak = ui.as_weak();
        move |text| picker::add_manual(&weak.unwrap(), &text)
    });
    ui.on_picker_browse({
        let weak = ui.as_weak();
        move || picker::browse(&weak.unwrap())
    });
    ui.on_remove_app(picker::remove);

    ui.on_clear_logs({
        let weak = ui.as_weak();
        move || {
            weak.unwrap()
                .set_logs(ModelRc::new(VecModel::<LogItem>::default()))
        }
    });

    ui.on_check_updates({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || {
            let ui = weak.unwrap();
            // Offline the request would be dropped and the button would stay on "Checking…".
            if ui.get_service_connected() {
                ui.set_checking_updates(true);
                let _ = tx.send(Command::CheckUpdates);
            }
        }
    });
    ui.on_install_package({
        let tx = cmd_tx.clone();
        move |slug| {
            if let Some(id) = PackageId::from_slug(&slug) {
                let _ = tx.send(Command::InstallPackage(id));
            }
        }
    });
    ui.on_remove_package({
        let tx = cmd_tx.clone();
        move |slug| {
            if let Some(id) = PackageId::from_slug(&slug) {
                let _ = tx.send(Command::RemovePackage(id));
            }
        }
    });

    // Easy mode + first-run wizard
    let prefs = std::rc::Rc::new(std::cell::RefCell::new(prefs::load()));
    ui.set_easy_mode(prefs.borrow().easy_mode);

    // Language: the system's unless chosen in Settings; switching applies at once.
    i18n::apply(&prefs.borrow().language);
    let language_names = || -> Vec<String> {
        i18n::choices()
            .iter()
            .map(|(code, name)| {
                if code.is_empty() {
                    tr(name)
                } else {
                    (*name).to_owned()
                }
            })
            .collect()
    };
    ui.set_language_names(string_model(language_names().iter().map(String::as_str)));
    ui.set_language_index(
        i18n::choices()
            .iter()
            .position(|(code, _)| *code == prefs.borrow().language)
            .unwrap_or(0) as i32,
    );
    ui.on_set_language({
        let weak = ui.as_weak();
        let prefs = prefs.clone();
        move |index| {
            let ui = weak.unwrap();
            let Some((code, _)) = i18n::choices().get(index as usize).copied() else {
                return;
            };
            {
                let mut p = prefs.borrow_mut();
                p.language = code.to_owned();
                prefs::save(&p);
            }
            i18n::apply(code);
            ui.set_language_index(index);
            ui.set_language_names(string_model(language_names().iter().map(String::as_str)));
            // Text built in Rust is refreshed from the current state.
            let profiles = PROFILES.with_borrow(|p| p.clone());
            apply_profiles(&ui, profiles);
        }
    });
    let show_log_settings = {
        let weak = ui.as_weak();
        move |p: &prefs::Prefs| {
            let ui = weak.unwrap();
            let dir = p.log_dir.clone().or_else(logfile::default_dir);
            ui.set_log_to_file(p.log_to_file);
            ui.set_log_dir(
                dir.map(|d| d.display().to_string())
                    .unwrap_or_default()
                    .into(),
            );
            ui.set_detailed_log(p.detailed_log);
            logfile::configure(p.log_dir(), p.log_folder());
            bridge::set_detailed_log(p.detailed_log);
        }
    };
    show_log_settings(&prefs.borrow());
    ui.on_set_detailed_log({
        let prefs = prefs.clone();
        let show = show_log_settings.clone();
        let tx = cmd_tx.clone();
        move |on| {
            let mut p = prefs.borrow_mut();
            p.detailed_log = on;
            prefs::save(&p);
            show(&p);
            let _ = tx.send(Command::SetDetailedLog(on));
        }
    });
    ui.on_set_log_to_file({
        let prefs = prefs.clone();
        let show = show_log_settings.clone();
        move |on| {
            let mut p = prefs.borrow_mut();
            p.log_to_file = on;
            prefs::save(&p);
            show(&p);
        }
    });
    ui.on_choose_log_dir({
        let prefs = prefs.clone();
        let show = show_log_settings.clone();
        move || {
            let start = prefs.borrow().log_dir().or_else(logfile::default_dir);
            let mut dialog = rfd::FileDialog::new().set_title(tr("Folder for DPIMech log files"));
            if let Some(start) = start {
                dialog = dialog.set_directory(start);
            }
            if let Some(dir) = dialog.pick_folder() {
                let mut p = prefs.borrow_mut();
                p.log_dir = Some(dir);
                p.log_to_file = true;
                prefs::save(&p);
                show(&p);
            }
        }
    });
    ui.on_open_link(|url| open_url(&url));

    // Problem report: collected when the dialog opens, re-saved with the user's note when sent.
    let report_state: std::rc::Rc<std::cell::RefCell<report::Open>> = Default::default();
    ui.set_report_email(dpimech_core::catalog::SUPPORT_EMAIL.into());
    ui.on_open_report({
        let weak = ui.as_weak();
        let state = report_state.clone();
        move || {
            let ui = weak.unwrap();
            let r = report::Report::collect(&ui);
            let saved = r.save("");
            ui.set_report_text(r.text("").into());
            ui.set_report_saved(
                saved
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
                    .into(),
            );
            ui.set_report_note(SharedString::new());
            ui.set_report_open(true);
            let _ = ui.show();
            *state.borrow_mut() = Some((r, saved));
        }
    });
    let send_report = {
        let weak = ui.as_weak();
        let state = report_state.clone();
        move |mailto: bool| {
            let ui = weak.unwrap();
            let note = ui.get_report_note().to_string();
            let state = state.borrow();
            let Some((r, saved)) = state.as_ref() else {
                return;
            };
            // The saved file should carry the note too.
            if let Some(path) = saved {
                let _ = std::fs::write(path, r.text(&note));
            }
            let url = if mailto {
                r.mailto_url(&note, saved.as_ref())
            } else {
                r.github_url(&note, saved.as_ref())
            };
            open_url(&url);
        }
    };
    ui.on_report_github({
        let send = send_report.clone();
        move || send(false)
    });
    ui.on_report_email_send(move || send_report(true));
    ui.on_report_folder({
        let state = report_state.clone();
        move || {
            if let Some((_, Some(path))) = state.borrow().as_ref()
                && let Some(dir) = path.parent()
            {
                open_url(&dir.display().to_string());
            }
        }
    });
    ui.on_open_log_dir({
        let prefs = prefs.clone();
        move || {
            if let Some(dir) = prefs.borrow().log_dir().or_else(logfile::default_dir) {
                let _ = std::fs::create_dir_all(&dir);
                open_url(&dir.display().to_string());
            }
        }
    });
    shortcutui::init(&ui);
    ui.on_open_shortcut({
        let weak = ui.as_weak();
        move |id| shortcutui::open(&weak.unwrap(), &id)
    });
    ui.on_shortcut_app_changed({
        let weak = ui.as_weak();
        move |index| shortcutui::app_changed(&weak.unwrap(), index)
    });
    ui.on_shortcut_create({
        let weak = ui.as_weak();
        move || shortcutui::create(&weak.unwrap())
    });
    ui.on_remove_all_shortcuts({
        let weak = ui.as_weak();
        move || shortcutui::remove_all(&weak.unwrap())
    });
    wizard::init(&ui, cmd_tx.clone());
    ui.on_open_wizard({
        let weak = ui.as_weak();
        move || wizard::open(&weak.unwrap(), false)
    });
    ui.on_wizard_choose_easy({
        let weak = ui.as_weak();
        let prefs = prefs.clone();
        move || {
            let ui = weak.unwrap();
            {
                let mut p = prefs.borrow_mut();
                p.easy_mode = true;
                p.onboarded = true;
                prefs::save(&p);
            }
            ui.set_easy_mode(true);
            ui.set_wizard_step(1);
        }
    });
    ui.on_wizard_choose_advanced({
        let weak = ui.as_weak();
        let prefs = prefs.clone();
        move || {
            let ui = weak.unwrap();
            {
                let mut p = prefs.borrow_mut();
                p.easy_mode = false;
                p.onboarded = true;
                prefs::save(&p);
            }
            ui.set_easy_mode(false);
            wizard::close(&ui);
        }
    });
    ui.on_set_easy_mode({
        let weak = ui.as_weak();
        let prefs = prefs.clone();
        move |on| {
            let ui = weak.unwrap();
            {
                let mut p = prefs.borrow_mut();
                p.easy_mode = on;
                p.onboarded = true;
                prefs::save(&p);
            }
            ui.set_easy_mode(on);
            if on && matches!(ui.get_page(), Page::Lab | Page::Engines | Page::Logs) {
                ui.set_page(Page::Dashboard);
            }
        }
    });
    ui.on_wizard_toggle_pack({
        let weak = ui.as_weak();
        move |i| wizard::toggle_pack(&weak.unwrap(), i)
    });
    ui.on_wizard_set_where({
        let weak = ui.as_weak();
        move |i| wizard::set_where(&weak.unwrap(), i)
    });
    ui.on_wizard_detect_isp({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || {
            let ui = weak.unwrap();
            if !ui.get_service_connected() {
                ui.set_wizard_isp_text(tr("This needs the background service.").into());
                return;
            }
            ui.set_wizard_isp_text(tr("Looking up…").into());
            let _ = tx.send(Command::DetectIsp);
        }
    });
    ui.on_install_service({
        let weak = ui.as_weak();
        move || {
            let ui = weak.unwrap();
            let show = |ui: &AppWindow, text: String| {
                ui.set_wizard_message(text.clone().into());
                ui.set_service_info(text.into());
            };
            let finished = {
                let weak = ui.as_weak();
                move |result: Result<(), String>| {
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        let text = match result {
                            Ok(()) => tr("The service is installed. Connecting…"),
                            Err(e) => trf!("Installing the service failed: {}", e),
                        };
                        ui.set_wizard_message(text.clone().into());
                        ui.set_service_info(text.into());
                    });
                }
            };
            match prefs::install_service_elevated(finished) {
                Ok(()) => show(&ui, tr("Installing… confirm the prompt. DPIMech connects by itself when the service is ready.")),
                Err(e) => show(&ui, trf!("Could not start the installer: {}", e)),
            }
        }
    });
    ui.on_wizard_next({
        let weak = ui.as_weak();
        move || wizard::next(&weak.unwrap())
    });
    ui.on_wizard_back({
        let weak = ui.as_weak();
        move || wizard::back(&weak.unwrap())
    });
    ui.on_wizard_retry({
        let weak = ui.as_weak();
        move || wizard::retry(&weak.unwrap())
    });
    ui.on_wizard_finish({
        let weak = ui.as_weak();
        move || {
            let ui = weak.unwrap();
            wizard::close(&ui);
            ui.set_page(Page::Dashboard);
        }
    });
    ui.on_wizard_close({
        let weak = ui.as_weak();
        move || wizard::close(&weak.unwrap())
    });
    if !prefs.borrow().onboarded {
        wizard::open(&ui, true);
    }

    labui::init(&ui);
    if let Some(engine) = labui::engine_at(0) {
        let _ = cmd_tx.send(Command::LabStrategies(engine));
    }
    ui.on_lab_engine_changed({
        let tx = cmd_tx.clone();
        move |index| {
            if let Some(engine) = labui::engine_at(index) {
                let _ = tx.send(Command::LabStrategies(engine));
            }
        }
    });
    ui.on_lab_toggle_pack({
        let weak = ui.as_weak();
        move |i| labui::toggle_pack(&weak.unwrap(), i)
    });
    ui.on_lab_refresh({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || {
            let ui = weak.unwrap();
            if let Some(engine) = labui::engine_at(ui.get_lab_engine_index()) {
                ui.set_lab_status(tr("Downloading online lists…").into());
                let _ = tx.send(Command::LabRefresh(engine));
            }
        }
    });
    ui.on_lab_detect_isp({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || {
            let ui = weak.unwrap();
            if !ui.get_service_connected() {
                ui.set_lab_isp_text(tr("This needs the background service.").into());
                return;
            }
            ui.set_lab_isp_text(tr("Looking up…").into());
            let _ = tx.send(Command::DetectIsp);
            // Reload so presets for the detected ISP get their star.
            if let Some(engine) = labui::engine_at(ui.get_lab_engine_index()) {
                let _ = tx.send(Command::LabStrategies(engine));
            }
        }
    });
    ui.on_lab_start({
        let weak = ui.as_weak();
        let tx = cmd_tx.clone();
        move || {
            let ui = weak.unwrap();
            match labui::request(&ui) {
                Ok(request) => {
                    let _ = tx.send(Command::StartLab(request));
                }
                Err(message) => ui.set_lab_status(message.into()),
            }
        }
    });
    ui.on_lab_cancel({
        let tx = cmd_tx.clone();
        move || {
            let _ = tx.send(Command::CancelLab);
        }
    });
    ui.on_lab_use({
        let weak = ui.as_weak();
        let engines = engines.clone();
        let set_routing_names = set_routing_names.clone();
        move |index| {
            let ui = weak.unwrap();
            let Some((engine, name, args, domains)) = labui::result_at(index) else {
                return;
            };
            let Some(engine_index) = engines.iter().position(|e| *e == engine) else {
                return;
            };
            let engine_index = engine_index as i32;
            set_routing_names(&ui, engine_index);
            // Proxy engines default to per-app routing; WinDivert engines are system-wide.
            let routing_index = if engine == EngineKind::ByeDpi {
                ui.get_per_app_index()
            } else {
                ui.get_system_wide_index()
            }
            .max(0);
            let mut draft = convert::empty_draft();
            draft.name = format!("{} ({name})", engine.display_name()).into();
            draft.engine_index = engine_index;
            draft.routing_index = routing_index;
            draft.args = args.into();
            draft.check_sites = convert::check_sites_for(&domains).join(", ").into();
            if engine != EngineKind::ByeDpi {
                draft.domains = domains.join("\n").into();
            }
            picker::load_draft(&[]);
            ui.set_draft(draft);
            ui.set_editor_error(SharedString::new());
            ui.set_page(Page::Editor);
        }
    });

    // Closing the window keeps the app in the tray; quitting happens from the tray menu.
    // Without a tray the window could not be brought back, so closing quits.
    ui.window().on_close_requested(|| {
        if tray::available() {
            CloseRequestResponse::HideWindow
        } else {
            let _ = slint::quit_event_loop();
            CloseRequestResponse::HideWindow
        }
    });
    // macOS only accepts a status-bar item once the application's event loop runs.
    #[cfg(target_os = "macos")]
    {
        let (weak, tx) = (ui.as_weak(), cmd_tx.clone());
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Err(e) = tray::create(weak, tx) {
                eprintln!("tray unavailable: {e:#}");
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    tray::create(ui.as_weak(), cmd_tx.clone())?;
    let _hotkey = hotkey::register(cmd_tx.clone());

    // Warm the app list so icons are ready when the editor opens.
    picker::refresh(ui.as_weak());

    // Development aid for screenshots: DPIMECH_DEBUG_PAGE=engines|logs|editor|picker|shortcut.
    #[cfg(debug_assertions)]
    match std::env::var("DPIMECH_DEBUG_PAGE").as_deref() {
        Ok("engines") => ui.set_page(Page::Engines),
        Ok("logs") => ui.set_page(Page::Logs),
        Ok("shortcut") => {
            let weak = ui.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                let id = std::env::var("DPIMECH_DEBUG_PROFILE").unwrap_or_default();
                shortcutui::open(&weak.unwrap(), &id);
            });
        }
        Ok(p @ ("editor" | "picker")) => {
            let open_picker = p == "picker";
            let weak = ui.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                let ui = weak.unwrap();
                let wanted = std::env::var("DPIMECH_DEBUG_PROFILE").ok();
                let first = PROFILES.with_borrow(|p| {
                    p.iter()
                        .find(|s| wanted.as_deref().is_none_or(|w| s.profile.id == w))
                        .map(|s| s.profile.id.clone())
                });
                match first {
                    Some(id) => ui.invoke_open_profile(id.into()),
                    None => ui.invoke_new_profile(),
                }
                if open_picker {
                    picker::open(&ui);
                }
            });
        }
        _ => {}
    }

    // Launched at sign-in with --minimized: stay in the tray until the user opens the window.
    if !std::env::args().any(|a| a == autostart::MINIMIZED_FLAG) {
        ui.show()?;
    } else {
        // The Linux tray comes up on its own thread; if it never does, show the window instead
        // of running invisibly.
        let weak = ui.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_secs(3), move || {
            if !tray::available()
                && let Some(ui) = weak.upgrade()
            {
                let _ = ui.show();
            }
        });
    }
    slint::run_event_loop_until_quit()?;
    Ok(())
}

thread_local! {
    static APP_UPDATE_URL: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

pub fn set_app_update(ui: &AppWindow, text: String, url: Option<String>) {
    ui.set_app_update_available(url.is_some());
    ui.set_app_update_text(text.into());
    APP_UPDATE_URL.set(url);
}

/// Updates the profile cards. Rows change in place while the same profiles are listed, so a card
/// keeps its open menu when a status update arrives; a new model only when profiles come or go.
pub fn apply_profiles(ui: &AppWindow, profiles: Vec<ProfileState>) {
    let items: Vec<ProfileItem> = profiles.iter().map(convert::profile_item).collect();
    let current = ui.get_profiles();
    let same_list = current.row_count() == items.len()
        && items
            .iter()
            .enumerate()
            .all(|(i, item)| current.row_data(i).is_some_and(|row| row.id == item.id));
    match current.as_any().downcast_ref::<VecModel<ProfileItem>>() {
        Some(model) if same_list => {
            for (i, item) in items.into_iter().enumerate() {
                if model.row_data(i).as_ref() != Some(&item) {
                    model.set_row_data(i, item);
                }
            }
        }
        _ => ui.set_profiles(ModelRc::new(VecModel::from(items))),
    }
    tray::update_profiles(&profiles);
    PROFILES.set(profiles);
}

fn set_domain_pack_names(ui: &AppWindow) {
    let packs = dpimech_core::catalog::domain_packs();
    ui.set_domain_pack_names(string_model(packs.iter().map(|p| p.name.as_str())));
}

/// The service sent its domain packs (possibly newer than the built-in ones).
pub fn apply_domain_packs(ui: &AppWindow, packs: Vec<dpimech_core::catalog::DomainPack>) {
    if dpimech_core::catalog::set_domain_packs(packs) {
        set_domain_pack_names(ui);
        labui::refresh_packs(ui);
        wizard::refresh_packs(ui);
    }
}

fn string_model<'a>(items: impl Iterator<Item = &'a str>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        items.map(SharedString::from).collect::<Vec<_>>(),
    ))
}

/// Opens a link in the user's browser.
fn open_url(url: &str) {
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(opener).arg(url).spawn() {
        eprintln!("could not open {url}: {e}");
    }
}
