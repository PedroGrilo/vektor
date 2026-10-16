//! Display mode enumeration and switching (EnumDisplaySettingsW / ChangeDisplaySettingsExW).
//! Modes are applied dynamically (no CDS_UPDATEREGISTRY): a crash or reboot brings
//! back the user's own mode.

use super::{DisplayInfo, DisplayMode};
use crate::error::{Error, Result};

#[cfg_attr(not(windows), allow(dead_code))]
/// Deduplicate and sort modes: largest resolution first, then highest refresh.
fn normalize(mut modes: Vec<DisplayMode>) -> Vec<DisplayMode> {
    modes.sort_by(|a, b| {
        (b.width * b.height, b.width, b.refresh).cmp(&(a.width * a.height, a.width, a.refresh))
    });
    modes.dedup();
    modes
}

pub fn list() -> Result<Vec<DisplayInfo>> {
    imp::list()
}

/// Apply a mode; returns the previous one.
pub fn set_mode(device: &str, w: u32, h: u32, hz: u32) -> Result<DisplayMode> {
    let wanted = DisplayMode { width: w, height: h, refresh: hz };
    let display = imp::list()?
        .into_iter()
        .find(|d| if device.is_empty() { d.primary } else { d.device.eq_ignore_ascii_case(device) })
        .ok_or_else(|| Error::Unavailable(format!("ecrã \"{device}\" não encontrado")))?;
    if display.current == wanted {
        return Ok(display.current);
    }
    if !display.modes.contains(&wanted) {
        return Err(Error::InvalidInput(format!(
            "o ecrã {} não suporta {}x{} a {} Hz",
            display.name, w, h, hz
        )));
    }
    imp::apply(&display.device, wanted)?;
    Ok(display.current)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_FULLSCREEN, CDS_TEST,
        DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE,
        DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
        ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    fn new_devmode() -> DEVMODEW {
        DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() }
    }

    fn mode_of(dm: &DEVMODEW) -> DisplayMode {
        DisplayMode { width: dm.dmPelsWidth, height: dm.dmPelsHeight, refresh: dm.dmDisplayFrequency }
    }

    pub fn list() -> Result<Vec<DisplayInfo>> {
        let mut out = Vec::new();
        for i in 0.. {
            let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), i, &mut dd, 0) }.as_bool() {
                break;
            }
            if !dd.StateFlags.contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) {
                continue;
            }
            let device = from_wide(&dd.DeviceName);
            let dev_w = wide(&device);

            // Monitor name: first child device of the adapter output.
            let mut mon = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            let name = if unsafe { EnumDisplayDevicesW(PCWSTR(dev_w.as_ptr()), 0, &mut mon, 0) }.as_bool() {
                from_wide(&mon.DeviceString)
            } else {
                from_wide(&dd.DeviceString)
            };

            let mut cur = new_devmode();
            if !unsafe { EnumDisplaySettingsW(PCWSTR(dev_w.as_ptr()), ENUM_CURRENT_SETTINGS, &mut cur) }.as_bool() {
                continue;
            }
            let mut modes = Vec::new();
            for n in 0.. {
                let mut dm = new_devmode();
                if !unsafe { EnumDisplaySettingsW(PCWSTR(dev_w.as_ptr()), ENUM_DISPLAY_SETTINGS_MODE(n), &mut dm) }
                    .as_bool()
                {
                    break;
                }
                if dm.dmBitsPerPel == 32 {
                    modes.push(mode_of(&dm));
                }
            }
            out.push(DisplayInfo {
                device,
                name: if name.trim().is_empty() { "Ecrã".into() } else { name },
                primary: dd.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE),
                current: mode_of(&cur),
                modes: normalize(modes),
            });
        }
        Ok(out)
    }

    pub fn apply(device: &str, m: DisplayMode) -> Result<()> {
        let dev_w = wide(device);
        let mut dm = new_devmode();
        dm.dmPelsWidth = m.width;
        dm.dmPelsHeight = m.height;
        dm.dmDisplayFrequency = m.refresh;
        dm.dmBitsPerPel = 32;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_BITSPERPEL;

        let test = unsafe { ChangeDisplaySettingsExW(PCWSTR(dev_w.as_ptr()), Some(&dm), None, CDS_TEST, None) };
        if test != DISP_CHANGE_SUCCESSFUL {
            return Err(Error::msg(format!("o Windows recusou o modo {}x{} @ {} Hz (código {})", m.width, m.height, m.refresh, test.0)));
        }
        // CDS_FULLSCREEN = temporary change, not written to the registry.
        let res = unsafe { ChangeDisplaySettingsExW(PCWSTR(dev_w.as_ptr()), Some(&dm), None, CDS_FULLSCREEN, None) };
        if res != DISP_CHANGE_SUCCESSFUL {
            return Err(Error::msg(format!("não foi possível mudar a resolução (código {})", res.0)));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn list() -> Result<Vec<DisplayInfo>> {
        Err(Error::Unsupported)
    }
    pub fn apply(_device: &str, _m: DisplayMode) -> Result<()> {
        Err(Error::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(width: u32, height: u32, refresh: u32) -> DisplayMode {
        DisplayMode { width, height, refresh }
    }

    #[test]
    fn dedupes_and_sorts_desc() {
        let out = normalize(vec![m(1280, 960, 144), m(1920, 1080, 60), m(1920, 1080, 144), m(1280, 960, 144), m(1920, 1080, 60)]);
        assert_eq!(out, vec![m(1920, 1080, 144), m(1920, 1080, 60), m(1280, 960, 144)]);
    }
}
