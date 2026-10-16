//! Digital vibrance / saturation through vendor user-mode APIs.
//!
//! - AMD: ADL (atiadlxx.dll / atiadlxy.dll), `ADL_Display_Color_Get/Set` with
//!   `ADL_DISPLAY_COLOR_SATURATION`. Prototypes from AMD's official ColorCaps sample
//!   (display-library/Sample/ColorCaps/main.cpp), constants and structs from
//!   include/adl_defines.h and include/adl_structures.h.
//! - NVIDIA: the DVC functions are not in NVIDIA's public nvapi_interface.h, so
//!   vibrance is reported as unavailable on NVIDIA instead of guessing IDs.
//!
//! The public scale is 0..100 with 50 = driver default; it maps piecewise-linearly
//! onto the vendor range [min, default, max].

use serde::Serialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VibranceInfo {
    pub supported: bool,
    /// "nvidia" | "amd" | "other"
    pub vendor: String,
    pub current: Option<i32>,
}

/// Vendor range for one display.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Range {
    min: i32,
    default: i32,
    max: i32,
}

/// 0..100 (50 = default) → vendor value.
#[cfg_attr(not(windows), allow(dead_code))]
fn to_vendor(level: i32, r: Range) -> i32 {
    let level = level.clamp(0, 100);
    if level <= 50 {
        r.min + ((r.default - r.min) as i64 * level as i64 / 50) as i32
    } else {
        r.default + ((r.max - r.default) as i64 * (level - 50) as i64 / 50) as i32
    }
}

/// Vendor value → 0..100 (50 = default), rounded.
#[cfg_attr(not(windows), allow(dead_code))]
fn from_vendor(value: i32, r: Range) -> i32 {
    let v = value.clamp(r.min, r.max);
    let scaled = |num: i64, den: i64| if den == 0 { 0 } else { ((num * 50 + den / 2) / den) as i32 };
    if v <= r.default {
        scaled((v - r.min) as i64, (r.default - r.min) as i64)
    } else {
        50 + scaled((v - r.default) as i64, (r.max - r.default) as i64)
    }
}

fn check_level(level: i32) -> Result<i32> {
    if (0..=100).contains(&level) {
        Ok(level)
    } else {
        Err(Error::InvalidInput("a vibrance tem de estar entre 0 e 100".into()))
    }
}

pub fn info() -> Result<VibranceInfo> {
    let vendor = imp::vendor();
    if vendor == "amd" {
        let current = imp::amd_get("").ok();
        return Ok(VibranceInfo { supported: current.is_some(), vendor: vendor.into(), current });
    }
    Ok(VibranceInfo { supported: false, vendor: vendor.into(), current: None })
}

pub fn preview(level: i32) -> Result<VibranceInfo> {
    set_level("", check_level(level)?)?;
    info()
}

pub fn reset() -> Result<VibranceInfo> {
    set_level("", 50)?;
    info()
}

/// Set vibrance (0..100) on `device` (empty = primary).
pub fn set_level(device: &str, level: i32) -> Result<()> {
    let level = check_level(level)?;
    match imp::vendor() {
        "amd" => imp::amd_set(device, level),
        "nvidia" => Err(Error::Unavailable(
            "vibrance NVIDIA não disponível (API não pública no NVAPI SDK)".into(),
        )),
        _ => Err(Error::Unavailable("nenhuma placa gráfica AMD ou NVIDIA compatível".into())),
    }
}

/// Current level (0..100) on `device` (empty = primary).
pub fn get_level(device: &str) -> Result<i32> {
    match imp::vendor() {
        "amd" => imp::amd_get(device),
        "nvidia" => Err(Error::Unavailable(
            "vibrance NVIDIA não disponível (API não pública no NVAPI SDK)".into(),
        )),
        _ => Err(Error::Unavailable("nenhuma placa gráfica AMD ou NVIDIA compatível".into())),
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::{c_char, c_void};
    use std::sync::{Mutex, OnceLock};

    // adl_defines.h
    const ADL_OK: i32 = 0; // line 109
    const ADL_MAX_PATH: usize = 256; // line 53
    const ADL_DISPLAY_COLOR_SATURATION: i32 = 1 << 2; // line 192
    const ADL_DISPLAY_DISPLAYINFO_DISPLAYCONNECTED: i32 = 0x0000_0001; // line 743
    const ADL_DISPLAY_DISPLAYINFO_DISPLAYMAPPED: i32 = 0x0000_0002; // line 744

    // adl_structures.h: AdapterInfo (Windows layout, line 42)
    #[repr(C)]
    struct AdapterInfo {
        i_size: i32,
        i_adapter_index: i32,
        str_udid: [c_char; ADL_MAX_PATH],
        i_bus_number: i32,
        i_device_number: i32,
        i_function_number: i32,
        i_vendor_id: i32,
        str_adapter_name: [c_char; ADL_MAX_PATH],
        str_display_name: [c_char; ADL_MAX_PATH],
        i_present: i32,
        i_exist: i32,
        str_driver_path: [c_char; ADL_MAX_PATH],
        str_driver_path_ext: [c_char; ADL_MAX_PATH],
        str_pnp_string: [c_char; ADL_MAX_PATH],
        i_os_display_index: i32,
    }

    // adl_structures.h: ADLDisplayID (line 525)
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct AdlDisplayId {
        i_display_logical_index: i32,
        i_display_physical_index: i32,
        i_display_logical_adapter_index: i32,
        i_display_physical_adapter_index: i32,
    }

    // adl_structures.h: ADLDisplayInfo (line 554)
    #[repr(C)]
    struct AdlDisplayInfo {
        display_id: AdlDisplayId,
        i_display_controller_index: i32,
        str_display_name: [c_char; ADL_MAX_PATH],
        str_display_manufacturer_name: [c_char; ADL_MAX_PATH],
        i_display_type: i32,
        i_display_output_type: i32,
        i_display_connector: i32,
        i_display_info_mask: i32,
        i_display_info_value: i32,
    }

    // Prototypes from Sample/ColorCaps/main.cpp.
    type MallocCb = unsafe extern "system" fn(i32) -> *mut c_void;
    type MainControlCreate = unsafe extern "C" fn(MallocCb, i32) -> i32;
    type NumberOfAdaptersGet = unsafe extern "C" fn(*mut i32) -> i32;
    type AdapterInfoGet = unsafe extern "C" fn(*mut AdapterInfo, i32) -> i32;
    type DisplayInfoGet = unsafe extern "C" fn(i32, *mut i32, *mut *mut AdlDisplayInfo, i32) -> i32;
    type ColorGet = unsafe extern "C" fn(i32, i32, i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> i32;
    type ColorSet = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

    struct Adl {
        _lib: libloading::Library,
        number_of_adapters_get: NumberOfAdaptersGet,
        adapter_info_get: AdapterInfoGet,
        display_info_get: DisplayInfoGet,
        color_get: ColorGet,
        color_set: ColorSet,
    }

    // ADL v1 is not thread-safe; every call goes through this lock.
    static ADL: OnceLock<Option<Mutex<Adl>>> = OnceLock::new();
    static NVAPI_PRESENT: OnceLock<bool> = OnceLock::new();

    // Allocation callback for ADL. A size header lets us free buffers ADL returns.
    const HDR: usize = 16;
    unsafe extern "system" fn adl_alloc(size: i32) -> *mut c_void {
        let Ok(size) = usize::try_from(size) else { return std::ptr::null_mut() };
        let Ok(layout) = std::alloc::Layout::from_size_align(size + HDR, HDR) else { return std::ptr::null_mut() };
        // SAFETY: layout has non-zero size (HDR > 0).
        let p = unsafe { std::alloc::alloc_zeroed(layout) };
        if p.is_null() {
            return p.cast();
        }
        // SAFETY: p points to at least HDR bytes, aligned to 16.
        unsafe {
            (p as *mut usize).write(size);
            p.add(HDR).cast()
        }
    }

    unsafe fn adl_free(ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }
        // SAFETY: ptr was produced by adl_alloc, so the header sits HDR bytes before it.
        unsafe {
            let base = (ptr as *mut u8).sub(HDR);
            let size = (base as *const usize).read();
            std::alloc::dealloc(base, std::alloc::Layout::from_size_align_unchecked(size + HDR, HDR));
        }
    }

    fn load_adl() -> Option<Mutex<Adl>> {
        // SAFETY: loading a vendor DLL by name; symbols are checked before use.
        let lib = unsafe { libloading::Library::new("atiadlxx.dll") }
            .or_else(|_| unsafe { libloading::Library::new("atiadlxy.dll") })
            .ok()?;
        // SAFETY: symbol types match the prototypes in AMD's ColorCaps sample.
        unsafe {
            let create = *lib.get::<MainControlCreate>(b"ADL_Main_Control_Create\0").ok()?;
            let adl = Adl {
                number_of_adapters_get: *lib.get(b"ADL_Adapter_NumberOfAdapters_Get\0").ok()?,
                adapter_info_get: *lib.get(b"ADL_Adapter_AdapterInfo_Get\0").ok()?,
                display_info_get: *lib.get(b"ADL_Display_DisplayInfo_Get\0").ok()?,
                color_get: *lib.get(b"ADL_Display_Color_Get\0").ok()?,
                color_set: *lib.get(b"ADL_Display_Color_Set\0").ok()?,
                _lib: lib,
            };
            // 1 = enumerate only connected adapters (as in the sample).
            if create(adl_alloc, 1) != ADL_OK {
                return None;
            }
            Some(Mutex::new(adl))
        }
    }

    fn adl() -> Option<&'static Mutex<Adl>> {
        ADL.get_or_init(load_adl).as_ref()
    }

    fn nvapi_present() -> bool {
        *NVAPI_PRESENT.get_or_init(|| {
            // SAFETY: only checks the DLL can be loaded; no functions are called.
            unsafe { libloading::Library::new("nvapi64.dll") }.is_ok()
        })
    }

    pub fn vendor() -> &'static str {
        if adl().is_some() {
            "amd"
        } else if nvapi_present() {
            "nvidia"
        } else {
            "other"
        }
    }

    fn cstr(buf: &[c_char]) -> String {
        let bytes: Vec<u8> = buf.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn primary_device() -> Option<String> {
        super::super::modes::list().ok()?.into_iter().find(|d| d.primary).map(|d| d.device)
    }

    /// (adapter index, display logical index) of the display shown as `device`.
    fn find_display(a: &Adl, device: &str) -> Result<(i32, i32)> {
        let target = if device.is_empty() {
            primary_device().ok_or_else(|| Error::Unavailable("ecrã principal não encontrado".into()))?
        } else {
            device.to_string()
        };
        let mut n = 0i32;
        // SAFETY: valid out-pointer.
        if unsafe { (a.number_of_adapters_get)(&mut n) } != ADL_OK || n <= 0 {
            return Err(Error::Unavailable("nenhum adaptador AMD encontrado".into()));
        }
        let mut infos: Vec<AdapterInfo> = (0..n)
            .map(|_| {
                // SAFETY: AdapterInfo is plain old data; all-zero is a valid value.
                let mut ai: AdapterInfo = unsafe { std::mem::zeroed() };
                ai.i_size = std::mem::size_of::<AdapterInfo>() as i32;
                ai
            })
            .collect();
        let bytes = (std::mem::size_of::<AdapterInfo>() * infos.len()) as i32;
        // SAFETY: buffer holds n AdapterInfo structs of `bytes` total size.
        if unsafe { (a.adapter_info_get)(infos.as_mut_ptr(), bytes) } != ADL_OK {
            return Err(Error::msg("ADL: falha ao ler os adaptadores"));
        }
        for ai in &infos {
            if !cstr(&ai.str_display_name).eq_ignore_ascii_case(&target) {
                continue;
            }
            let adapter = ai.i_adapter_index;
            let mut count = 0i32;
            let mut list: *mut AdlDisplayInfo = std::ptr::null_mut();
            // SAFETY: ADL allocates `list` with adl_alloc; freed below.
            let rc = unsafe { (a.display_info_get)(adapter, &mut count, &mut list, 0) };
            if rc != ADL_OK || list.is_null() {
                continue;
            }
            // SAFETY: ADL returned `count` entries at `list`.
            let displays = unsafe { std::slice::from_raw_parts(list, count.max(0) as usize) };
            let wanted = ADL_DISPLAY_DISPLAYINFO_DISPLAYCONNECTED | ADL_DISPLAY_DISPLAYINFO_DISPLAYMAPPED;
            let found = displays
                .iter()
                .find(|d| {
                    d.i_display_info_value & wanted == wanted
                        && d.display_id.i_display_logical_adapter_index == adapter
                })
                .map(|d| d.display_id.i_display_logical_index);
            // SAFETY: list came from adl_alloc.
            unsafe { adl_free(list.cast()) };
            if let Some(idx) = found {
                return Ok((adapter, idx));
            }
        }
        Err(Error::Unavailable(format!("ecrã {target} não está ligado a uma placa AMD")))
    }

    fn range_and_current(a: &Adl, adapter: i32, display: i32) -> Result<(Range, i32)> {
        let (mut cur, mut def, mut min, mut max, mut step) = (0, 0, 0, 0, 0);
        // SAFETY: valid out-pointers for the documented signature.
        let rc = unsafe {
            (a.color_get)(adapter, display, ADL_DISPLAY_COLOR_SATURATION, &mut cur, &mut def, &mut min, &mut max, &mut step)
        };
        if rc != ADL_OK || max <= min || def < min || def > max {
            return Err(Error::Unavailable("saturação não suportada neste ecrã".into()));
        }
        Ok((Range { min, default: def, max }, cur))
    }

    pub fn amd_get(device: &str) -> Result<i32> {
        let a = adl().ok_or(Error::Unsupported)?.lock().map_err(|_| Error::msg("ADL ocupado"))?;
        let (adapter, display) = find_display(&a, device)?;
        let (range, cur) = range_and_current(&a, adapter, display)?;
        Ok(from_vendor(cur, range))
    }

    pub fn amd_set(device: &str, level: i32) -> Result<()> {
        let a = adl().ok_or(Error::Unsupported)?.lock().map_err(|_| Error::msg("ADL ocupado"))?;
        let (adapter, display) = find_display(&a, device)?;
        let (range, _) = range_and_current(&a, adapter, display)?;
        // SAFETY: plain value call with indices obtained from ADL.
        let rc = unsafe { (a.color_set)(adapter, display, ADL_DISPLAY_COLOR_SATURATION, to_vendor(level, range)) };
        if rc != ADL_OK {
            return Err(Error::msg(format!("ADL recusou a alteração da saturação (código {rc})")));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn vendor() -> &'static str {
        "other"
    }
    pub fn amd_get(_device: &str) -> Result<i32> {
        Err(Error::Unsupported)
    }
    pub fn amd_set(_device: &str, _level: i32) -> Result<()> {
        Err(Error::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Range = Range { min: 0, default: 100, max: 200 };
    const ASYM: Range = Range { min: -50, default: 0, max: 150 };

    #[test]
    fn maps_endpoints_and_default() {
        assert_eq!(to_vendor(0, R), 0);
        assert_eq!(to_vendor(50, R), 100);
        assert_eq!(to_vendor(100, R), 200);
        assert_eq!(to_vendor(75, ASYM), 75);
        assert_eq!(to_vendor(25, ASYM), -25);
    }

    #[test]
    fn roundtrips_and_clamps() {
        for l in 0..=100 {
            assert_eq!(from_vendor(to_vendor(l, R), R), l);
        }
        assert_eq!(from_vendor(999, R), 100);
        assert_eq!(from_vendor(-5, R), 0);
        assert_eq!(to_vendor(150, R), 200);
    }

    #[test]
    fn rejects_out_of_range_level() {
        assert!(check_level(101).is_err());
        assert!(check_level(-1).is_err());
    }
}
