//! Windows service start type / running state via the Service Control Manager.

use crate::error::Result;
use crate::journal::{self, Hive, RegValue, Undo};

/// Start type values (same numbers as the `Start` registry value).
pub const START_DEMAND: u32 = 3;
pub const START_DISABLED: u32 = 4;

/// Restore start type, then start the service if it was running before.
pub fn restore(name: &str, start: u32, running: bool) -> Result<()> {
    imp::set_start_type(name, start)?;
    if running {
        imp::start(name)?;
    }
    Ok(())
}

/// Current start type, read from `HKLM\SYSTEM\CurrentControlSet\Services\<name>\Start`.
pub fn start_type(name: &str) -> Result<Option<u32>> {
    let key = format!(r"SYSTEM\CurrentControlSet\Services\{name}");
    Ok(match crate::win::registry::read(Hive::Hklm, &key, "Start")? {
        Some(RegValue::Dword(v)) => Some(v),
        _ => None,
    })
}

pub fn exists(name: &str) -> bool {
    matches!(start_type(name), Ok(Some(_)))
}

/// Disable and stop a service, journaled under `tweak_id`.
pub fn disable_journaled(tweak_id: &str, name: &str) -> Result<()> {
    let previous_start = start_type(name)?.unwrap_or(START_DEMAND);
    let was_running = imp::is_running(name)?;
    journal::record(tweak_id, Undo::Service { name: name.to_string(), previous_start, was_running })?;
    imp::set_start_type(name, START_DISABLED)?;
    if was_running {
        imp::stop(name)?;
    }
    Ok(())
}

#[cfg(windows)]
mod imp {
    use windows::Win32::System::Services::{
        ChangeServiceConfigW, CloseServiceHandle, ControlService, ENUM_SERVICE_TYPE, OpenSCManagerW,
        OpenServiceW, QueryServiceStatus, SC_HANDLE, SC_MANAGER_CONNECT, SERVICE_CHANGE_CONFIG,
        SERVICE_CONTROL_STOP, SERVICE_ERROR, SERVICE_NO_CHANGE, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
        SERVICE_START, SERVICE_START_TYPE, SERVICE_STATUS, SERVICE_STOP, SERVICE_STOPPED, StartServiceW,
    };
    use windows::core::{HSTRING, PCWSTR};

    use crate::error::{Error, Result};

    /// Closes the SCM/service handles on drop.
    struct Handles {
        scm: SC_HANDLE,
        svc: SC_HANDLE,
    }

    impl Drop for Handles {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseServiceHandle(self.svc);
                let _ = CloseServiceHandle(self.scm);
            }
        }
    }

    fn open(name: &str, access: u32) -> Result<Handles> {
        let err = |e: windows::core::Error| Error::msg(format!("Serviço {name}: {}", e.message()));
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT).map_err(err)?;
            match OpenServiceW(scm, &HSTRING::from(name), access) {
                Ok(svc) => Ok(Handles { scm, svc }),
                Err(e) => {
                    let _ = CloseServiceHandle(scm);
                    Err(err(e))
                }
            }
        }
    }

    fn status(h: &Handles) -> Result<SERVICE_STATUS> {
        let mut st = SERVICE_STATUS::default();
        unsafe { QueryServiceStatus(h.svc, &mut st) }
            .map_err(|e| Error::msg(format!("Estado do serviço: {}", e.message())))?;
        Ok(st)
    }

    pub fn is_running(name: &str) -> Result<bool> {
        let h = open(name, SERVICE_QUERY_STATUS)?;
        Ok(status(&h)?.dwCurrentState == SERVICE_RUNNING)
    }

    pub fn set_start_type(name: &str, start: u32) -> Result<()> {
        let h = open(name, SERVICE_CHANGE_CONFIG)?;
        unsafe {
            ChangeServiceConfigW(
                h.svc,
                ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
                SERVICE_START_TYPE(start),
                SERVICE_ERROR(SERVICE_NO_CHANGE),
                PCWSTR::null(),
                PCWSTR::null(),
                None,
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
            )
        }
        .map_err(|e| Error::msg(format!("Serviço {name}: {}", e.message())))
    }

    pub fn stop(name: &str) -> Result<()> {
        let h = open(name, SERVICE_STOP | SERVICE_QUERY_STATUS)?;
        if status(&h)?.dwCurrentState == SERVICE_STOPPED {
            return Ok(());
        }
        let mut st = SERVICE_STATUS::default();
        unsafe { ControlService(h.svc, SERVICE_CONTROL_STOP, &mut st) }
            .map_err(|e| Error::msg(format!("Parar serviço {name}: {}", e.message())))
    }

    pub fn start(name: &str) -> Result<()> {
        let h = open(name, SERVICE_START | SERVICE_QUERY_STATUS)?;
        if status(&h)?.dwCurrentState == SERVICE_RUNNING {
            return Ok(());
        }
        unsafe { StartServiceW(h.svc, None) }
            .map_err(|e| Error::msg(format!("Iniciar serviço {name}: {}", e.message())))
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::error::{Error, Result};

    pub fn is_running(_: &str) -> Result<bool> {
        Err(Error::Unsupported)
    }
    pub fn set_start_type(_: &str, _: u32) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub fn stop(_: &str) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub fn start(_: &str) -> Result<()> {
        Err(Error::Unsupported)
    }
}
