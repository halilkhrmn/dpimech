//! Ties engine processes to the service's lifetime.
//!
//! `kill_on_drop` only helps on a clean shutdown. On Windows every child is added to a job
//! object with KILL_ON_JOB_CLOSE, so if the service crashes or is killed, the kernel closes
//! the job handle and terminates the engines too — no orphaned ciadpi/ProxiFyre.
//! On Linux each child asks the kernel for SIGKILL when its parent dies (`PR_SET_PDEATHSIG`);
//! under systemd `KillMode=control-group` covers the same case a second time.

use tokio::process::{Child, Command};

/// Must be called on every engine command before it is spawned.
#[cfg(target_os = "linux")]
pub fn contain(cmd: &mut Command) {
    // The signal fires when the *thread* that forked exits. Engines are spawned from tokio
    // worker threads, which live as long as the runtime, i.e. the service.
    let parent = std::process::id() as libc::pid_t;
    // SAFETY: only async-signal-safe calls between fork and exec.
    unsafe {
        cmd.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            // The service may have died between fork and prctl.
            if libc::getppid() != parent {
                libc::_exit(1);
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
pub fn contain(_cmd: &mut Command) {}

#[cfg(windows)]
pub fn adopt(child: &Child) {
    use std::sync::OnceLock;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    // The handle is intentionally never closed: closing it is what kills the children.
    static JOB: OnceLock<usize> = OnceLock::new();
    let job = *JOB.get_or_init(|| {
        // SAFETY: plain Win32 calls with valid, fully-initialised arguments.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return 0;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            job as usize
        }
    });
    if job == 0 {
        tracing::warn!("could not create job object; engines may outlive a crashed service");
        return;
    }
    if let Some(process) = child.raw_handle() {
        // SAFETY: both handles are valid for the duration of the call.
        let ok = unsafe { AssignProcessToJobObject(job as HANDLE, process as HANDLE) };
        if ok == 0 {
            tracing::warn!(
                "could not add engine to job object: {}",
                std::io::Error::last_os_error()
            );
        }
    }
}

#[cfg(not(windows))]
pub fn adopt(_child: &Child) {}
