//! `VektorMemory` service: runtime (SCM-launched) and install/uninstall/query (app side).

use crate::error::Result;

#[cfg(windows)]
mod imp {
    use std::ffi::OsString;
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::time::{Duration, Instant};

    use windows_service::service::{
        ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept, ServiceErrorControl,
        ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod, ServiceInfo, ServiceStartType,
        ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
    use windows_service::{define_windows_service, service_dispatcher};

    use super::super::{
        SERVICE_ARG, SERVICE_DESCRIPTION, SERVICE_DISPLAY, SERVICE_NAME, load_config, load_counters, now_iso8601,
        nt, save_counters, should_purge,
    };
    use crate::error::{Error, Result};

    const ERROR_SERVICE_DOES_NOT_EXIST: i32 = 1060;
    const MIN_PURGE_GAP: Duration = Duration::from_secs(10);
    /// The heartbeat is only for the UI, so it is written at most this often
    /// (purges and error changes are written immediately).
    const HEARTBEAT_EVERY: Duration = Duration::from_secs(30);

    fn win_err(e: windows_service::Error) -> Error {
        Error::msg(format!("Serviço {SERVICE_NAME}: {e}"))
    }

    // ---------- runtime ----------

    define_windows_service!(ffi_service_main, service_main);

    pub fn run_as_service() {
        // Fails when not started by the SCM; nothing useful to do then.
        let _ = service_dispatcher::start(SERVICE_NAME, ffi_service_main);
    }

    fn service_main(_args: Vec<OsString>) {
        let _ = run_service();
    }

    fn status(state: ServiceState, accept: ServiceControlAccept) -> ServiceStatus {
        ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        }
    }

    fn run_service() -> windows_service::Result<()> {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = service_control_handler::register(SERVICE_NAME, move |ev| match ev {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;
        handle.set_service_status(status(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        ))?;

        let mut last_purge: Option<Instant> = None;
        let mut last_saved: Option<Instant> = None;
        let mut counters = load_counters();
        loop {
            let cfg = load_config();
            let before = counters.clone();
            // Every failure is recorded: a service that silently skips its work is
            // indistinguishable from one with nothing to do.
            let outcome = nt::memory_lists().and_then(|lists| {
                let gap_ok = last_purge.is_none_or(|t| t.elapsed() >= MIN_PURGE_GAP);
                if gap_ok && should_purge(&cfg, lists.standby_bytes, lists.free_bytes) {
                    last_purge = Some(Instant::now());
                    nt::purge_standby_list()?;
                    counters.purge_count += 1;
                    counters.last_purge = Some(now_iso8601());
                }
                Ok(())
            });
            counters.last_error = outcome.err().map(|e| e.to_string());
            let changed = counters != before;
            if changed || last_saved.is_none_or(|t| t.elapsed() >= HEARTBEAT_EVERY) {
                counters.last_check = Some(now_iso8601());
                let _ = save_counters(&counters);
                last_saved = Some(Instant::now());
            }
            match stop_rx.recv_timeout(Duration::from_secs(cfg.interval_sec)) {
                Err(RecvTimeoutError::Timeout) => continue,
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        handle.set_service_status(status(ServiceState::Stopped, ServiceControlAccept::empty()))?;
        Ok(())
    }

    // ---------- management (from the elevated app) ----------

    fn manager(access: ServiceManagerAccess) -> Result<ServiceManager> {
        ServiceManager::local_computer(None::<&str>, access).map_err(win_err)
    }

    fn is_missing(e: &windows_service::Error) -> bool {
        matches!(e, windows_service::Error::Winapi(io) if io.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST))
    }

    pub fn query() -> Result<(bool, bool)> {
        let m = manager(ServiceManagerAccess::CONNECT)?;
        match m.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
            Ok(s) => {
                let st = s.query_status().map_err(win_err)?;
                Ok((true, st.current_state == ServiceState::Running))
            }
            Err(e) if is_missing(&e) => Ok((false, false)),
            Err(e) => Err(win_err(e)),
        }
    }

    /// The service runs as LocalSystem, so its binary must not be user-writable:
    /// only allow installing from Program Files (release builds).
    fn check_exe_location(exe: &std::path::Path) -> Result<()> {
        if cfg!(debug_assertions) {
            return Ok(());
        }
        let exe = exe.to_string_lossy().to_lowercase();
        let allowed = ["ProgramFiles", "ProgramW6432"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .any(|pf| exe.starts_with(&format!("{}\\", pf.to_lowercase().trim_end_matches('\\'))));
        if allowed {
            Ok(())
        } else {
            Err(Error::msg("O serviço só pode ser instalado com o Vektor instalado em Programas (Program Files)"))
        }
    }

    pub fn install() -> Result<()> {
        let exe = std::env::current_exe()?;
        check_exe_location(&exe)?;
        let m = manager(ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)?;
        let svc = match m.open_service(
            SERVICE_NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::START | ServiceAccess::CHANGE_CONFIG,
        ) {
            Ok(s) => s,
            Err(e) if is_missing(&e) => {
                let info = ServiceInfo {
                    name: SERVICE_NAME.into(),
                    display_name: SERVICE_DISPLAY.into(),
                    service_type: ServiceType::OWN_PROCESS,
                    start_type: ServiceStartType::AutoStart,
                    error_control: ServiceErrorControl::Normal,
                    executable_path: exe,
                    launch_arguments: vec![SERVICE_ARG.into()],
                    dependencies: vec![],
                    account_name: None, // LocalSystem
                    account_password: None,
                };
                m.create_service(
                    &info,
                    ServiceAccess::QUERY_STATUS | ServiceAccess::START | ServiceAccess::CHANGE_CONFIG,
                )
                .map_err(win_err)?
            }
            Err(e) => return Err(win_err(e)),
        };
        svc.set_description(SERVICE_DESCRIPTION).map_err(win_err)?;
        // Restart after a crash instead of staying down until the next boot.
        // Best effort: the service works without it.
        let restart = ServiceAction { action_type: ServiceActionType::Restart, delay: Duration::from_secs(5) };
        let _ = svc.update_failure_actions(ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(24 * 60 * 60)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![restart.clone(), restart.clone(), restart]),
        });
        if svc.query_status().map_err(win_err)?.current_state != ServiceState::Running {
            svc.start::<&str>(&[]).map_err(win_err)?;
        }
        Ok(())
    }

    pub fn uninstall() -> Result<()> {
        let m = manager(ServiceManagerAccess::CONNECT)?;
        let svc = match m.open_service(
            SERVICE_NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
        ) {
            Ok(s) => s,
            Err(e) if is_missing(&e) => return Ok(()),
            Err(e) => return Err(win_err(e)),
        };
        if svc.query_status().map_err(win_err)?.current_state != ServiceState::Stopped {
            let _ = svc.stop();
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                match svc.query_status() {
                    Ok(st) if st.current_state == ServiceState::Stopped => break,
                    _ => std::thread::sleep(Duration::from_millis(250)),
                }
            }
        }
        svc.delete().map_err(win_err)?;
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::error::{Error, Result};

    pub fn run_as_service() {}

    pub fn query() -> Result<(bool, bool)> {
        Err(Error::Unsupported)
    }

    pub fn install() -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn uninstall() -> Result<()> {
        Err(Error::Unsupported)
    }
}

/// Entry point when started by the SCM with `--memory-service`.
pub fn run_as_service() {
    imp::run_as_service()
}

/// (installed, running)
pub(crate) fn query() -> Result<(bool, bool)> {
    imp::query()
}

pub(crate) fn install() -> Result<()> {
    imp::install()
}

pub(crate) fn uninstall() -> Result<()> {
    imp::uninstall()
}
