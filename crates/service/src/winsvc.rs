//! Windows Service Control Manager integration.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use dpimech_core::paths::DataDir;
use windows_service::service::{
    ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
    ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};

const SERVICE_NAME: &str = "dpimech";
const DISPLAY_NAME: &str = "DPIMech Service";

static DATA_DIR: OnceLock<DataDir> = OnceLock::new();

/// Installs or upgrades the service. The binary is copied to Program Files first: a service
/// running as SYSTEM must never execute from a folder standard users can write to.
pub fn install(data: &DataDir) -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;
    let access = ServiceAccess::QUERY_STATUS
        | ServiceAccess::STOP
        | ServiceAccess::START
        | ServiceAccess::CHANGE_CONFIG;
    remove_legacy_service(&manager);
    crate::migrate::move_data_dir(data);
    let existing = manager.open_service(SERVICE_NAME, access).ok();
    if let Some(service) = &existing {
        stop_and_wait(service)?;
    }

    let exe = install_binary()?;
    crate::acl::harden_data_dir(&data.root)?;
    println!("data directory secured: {}", data.root.display());

    let info = ServiceInfo {
        name: SERVICE_NAME.into(),
        display_name: DISPLAY_NAME.into(),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe.clone(),
        launch_arguments: vec![OsString::from("service")],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };
    let service = match existing {
        Some(service) => {
            service.change_config(&info)?;
            service
        }
        None => manager.create_service(&info, access)?,
    };
    service.set_description("Runs DPI bypass engines for DPIMech.")?;
    // If the service itself ever crashes, Windows brings it back (engines die with it).
    service.update_failure_actions(ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(24 * 60 * 60)),
        reboot_msg: None,
        command: None,
        actions: Some(vec![
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_secs(5),
            };
            3
        ]),
    })?;
    service.start::<&str>(&[])?;
    println!("service installed and started from {}", exe.display());
    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
    )?;
    stop_and_wait(&service)?;
    service.delete()?;
    drop(service);
    // Best effort: fails harmlessly if uninstall runs from the installed copy itself.
    let _ = std::fs::remove_file(install_dir().join("dpimech-service.exe"));
    println!("service removed (profiles and engines in ProgramData were kept)");
    Ok(())
}

/// 0.1.x ran as the "dpimngr" service from Program Files\dpimngr. Stopping it also stops its
/// engines (job object), which releases the old data directory for the move.
fn remove_legacy_service(manager: &ServiceManager) {
    use crate::migrate::LEGACY;

    let access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    if let Ok(service) = manager.open_service(LEGACY, access) {
        let _ = stop_and_wait(&service);
        match service.delete() {
            Ok(()) => println!("removed the old {LEGACY} service"),
            Err(e) => println!("could not remove the old {LEGACY} service: {e}"),
        }
    }
    let base = std::env::var_os("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".into());
    let old_dir = PathBuf::from(base).join(LEGACY);
    let _ = std::fs::remove_file(old_dir.join(format!("{LEGACY}-service.exe")));
    let _ = std::fs::remove_dir(&old_dir);
    crate::firewall::remove_legacy_rule();
}

fn install_dir() -> PathBuf {
    let base = std::env::var_os("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".into());
    PathBuf::from(base).join("dpimech")
}

fn install_binary() -> anyhow::Result<PathBuf> {
    let current = std::env::current_exe()?;
    let target = install_dir().join("dpimech-service.exe");
    if current == target {
        return Ok(target);
    }
    std::fs::create_dir_all(install_dir())?;
    // The old copy may stay locked for a moment after the service stops.
    let mut last_err = None;
    for _ in 0..20 {
        match std::fs::copy(&current, &target) {
            Ok(_) => return Ok(target),
            Err(e) => last_err = Some(e),
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(last_err.unwrap().into())
}

fn stop_and_wait(service: &windows_service::service::Service) -> anyhow::Result<()> {
    if service.query_status()?.current_state != ServiceState::Stopped {
        let _ = service.stop();
        for _ in 0..100 {
            if service.query_status()?.current_state == ServiceState::Stopped {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    Ok(())
}

pub fn run_dispatcher(data: DataDir) -> anyhow::Result<()> {
    let _ = DATA_DIR.set(data);
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_args: Vec<OsString>) {
    if let Err(e) = run_service() {
        tracing::error!("service failed: {e:#}");
    }
}

fn run_service() -> anyhow::Result<()> {
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let mut stop_tx = Some(stop_tx);
    let handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            if let Some(tx) = stop_tx.take() {
                let _ = tx.send(());
            }
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let status_handle = service_control_handler::register(SERVICE_NAME, handler)?;
    let set_state = |state, accept| {
        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(5),
            process_id: None,
        })
    };

    set_state(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
    )?;
    let data = DATA_DIR
        .get()
        .cloned()
        .expect("data dir set before dispatch");
    // Re-applied on every start in case someone loosened the permissions.
    if let Err(e) = crate::acl::harden_data_dir(&data.root) {
        tracing::error!("could not secure {}: {e:#}", data.root.display());
    }
    let result = crate::run_until(data, async {
        let _ = stop_rx.await;
    });
    set_state(ServiceState::Stopped, ServiceControlAccept::empty())?;
    result
}
