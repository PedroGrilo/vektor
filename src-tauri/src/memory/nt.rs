//! ntdll memory-list query and standby purge.

use crate::error::Result;

pub struct MemoryLists {
    pub free_bytes: u64,
    pub standby_bytes: u64,
    pub modified_bytes: u64,
}

/// `SYSTEM_MEMORY_LIST_INFORMATION` (phnt), all fields ULONG_PTR.
#[cfg_attr(not(windows), allow(dead_code))]
#[repr(C)]
#[derive(Default)]
struct SystemMemoryListInformation {
    zero_page_count: usize,
    free_page_count: usize,
    modified_page_count: usize,
    modified_no_write_page_count: usize,
    bad_page_count: usize,
    page_count_by_priority: [usize; 8],
    repurposed_pages_by_priority: [usize; 8],
    modified_page_count_page_file: usize,
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::sync::OnceLock;

    use windows::Win32::Foundation::{CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID};
    use windows::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::core::w;

    use super::{MemoryLists, SystemMemoryListInformation};
    use crate::error::{Error, Result};

    const SYSTEM_MEMORY_LIST_INFORMATION: u32 = 80;
    const MEMORY_PURGE_STANDBY_LIST: u32 = 4;
    // Page size is fixed at 4 KiB on x64 Windows.
    const PAGE_SIZE: u64 = 4096;

    type NtQuerySystemInformation = unsafe extern "system" fn(u32, *mut c_void, u32, *mut u32) -> i32;
    type NtSetSystemInformation = unsafe extern "system" fn(u32, *const c_void, u32) -> i32;

    struct Ntdll {
        _lib: libloading::Library,
        query: NtQuerySystemInformation,
        set: NtSetSystemInformation,
    }

    fn ntdll() -> Result<&'static Ntdll> {
        static NTDLL: OnceLock<Option<Ntdll>> = OnceLock::new();
        NTDLL
            .get_or_init(|| unsafe {
                let lib = libloading::Library::new("ntdll.dll").ok()?;
                let query = *lib.get::<NtQuerySystemInformation>("NtQuerySystemInformation").ok()?;
                let set = *lib.get::<NtSetSystemInformation>("NtSetSystemInformation").ok()?;
                Some(Ntdll { _lib: lib, query, set })
            })
            .as_ref()
            .ok_or_else(|| Error::Unavailable("ntdll.dll".into()))
    }

    pub fn memory_lists() -> Result<MemoryLists> {
        let nt = ntdll()?;
        let mut info = SystemMemoryListInformation::default();
        let mut ret = 0u32;
        let status = unsafe {
            (nt.query)(
                SYSTEM_MEMORY_LIST_INFORMATION,
                (&mut info as *mut SystemMemoryListInformation).cast(),
                std::mem::size_of::<SystemMemoryListInformation>() as u32,
                &mut ret,
            )
        };
        if status < 0 {
            return Err(Error::msg(format!("Não foi possível ler a memória (NTSTATUS 0x{:08X})", status as u32)));
        }
        let pages = |n: usize| n as u64 * PAGE_SIZE;
        Ok(MemoryLists {
            free_bytes: pages(info.zero_page_count + info.free_page_count),
            standby_bytes: info.page_count_by_priority.iter().map(|&n| pages(n)).sum(),
            modified_bytes: pages(info.modified_page_count),
        })
    }

    fn enable_profile_privilege() -> Result<()> {
        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token)
                .map_err(|e| Error::msg(format!("OpenProcessToken: {e}")))?;
            let mut luid = LUID::default();
            let result = LookupPrivilegeValueW(None, w!("SeProfileSingleProcessPrivilege"), &mut luid)
                .and_then(|_| {
                    let tp = TOKEN_PRIVILEGES {
                        PrivilegeCount: 1,
                        Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
                    };
                    AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None)
                });
            let not_assigned = GetLastError() == ERROR_NOT_ALL_ASSIGNED;
            let _ = CloseHandle(token);
            result.map_err(|e| Error::msg(format!("AdjustTokenPrivileges: {e}")))?;
            if not_assigned {
                return Err(Error::msg("Sem privilégio para limpar a memória (é preciso administrador)"));
            }
            Ok(())
        }
    }

    pub fn purge_standby_list() -> Result<()> {
        let nt = ntdll()?;
        enable_profile_privilege()?;
        let command = MEMORY_PURGE_STANDBY_LIST;
        let status = unsafe {
            (nt.set)(
                SYSTEM_MEMORY_LIST_INFORMATION,
                (&command as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            )
        };
        if status < 0 {
            return Err(Error::msg(format!("Falha ao limpar a lista standby (NTSTATUS 0x{:08X})", status as u32)));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::MemoryLists;
    use crate::error::{Error, Result};

    pub fn memory_lists() -> Result<MemoryLists> {
        Err(Error::Unsupported)
    }

    pub fn purge_standby_list() -> Result<()> {
        Err(Error::Unsupported)
    }
}

pub fn memory_lists() -> Result<MemoryLists> {
    imp::memory_lists()
}

pub fn purge_standby_list() -> Result<()> {
    imp::purge_standby_list()
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn struct_layout_matches_phnt() {
        assert_eq!(std::mem::size_of::<super::SystemMemoryListInformation>(), 176);
    }
}
