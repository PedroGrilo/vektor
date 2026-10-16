//! Digital vibrance only while CS2 is the foreground window.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::display::vibrance;

pub(crate) struct FocusLoop {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl FocusLoop {
    /// Poll the foreground window every 500 ms; apply `level` while `pid` has
    /// focus, `original` otherwise. Only calls the driver when the state flips.
    pub(crate) fn start(pid: u32, level: i32, original: i32) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::Builder::new()
            .name("vektor-vibrance".into())
            .spawn(move || {
                let mut applied: Option<bool> = None;
                while !flag.load(Ordering::Relaxed) {
                    let focused = foreground_pid() == Some(pid);
                    if applied != Some(focused) {
                        let target = if focused { level } else { original };
                        // Errors are ignored: next state change retries.
                        if vibrance::set_level("", target).is_ok() {
                            applied = Some(focused);
                        }
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
                if applied == Some(true) {
                    let _ = vibrance::set_level("", original);
                }
            })
            .ok();
        FocusLoop { stop, handle }
    }

    pub(crate) fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(windows)]
fn foreground_pid() -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

#[cfg(not(windows))]
fn foreground_pid() -> Option<u32> {
    None
}
