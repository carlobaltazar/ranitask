use crate::player;
use crate::win32_helpers::wide;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use winapi::shared::windef::HWND;
use winapi::um::wingdi::GetPixel;
use winapi::um::winuser::*;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

const CLR_INVALID: u32 = 0xFFFFFFFF;
const COLOR_TOLERANCE: u32 = 48; // Manhattan distance across BGR channels

#[inline]
fn color_dist(a: u32, b: u32) -> u32 {
    let ar = (a & 0xFF) as i32;
    let br = (b & 0xFF) as i32;
    let ag = ((a >> 8) & 0xFF) as i32;
    let bg = ((b >> 8) & 0xFF) as i32;
    let ab = ((a >> 16) & 0xFF) as i32;
    let bb = ((b >> 16) & 0xFF) as i32;
    (ar - br).unsigned_abs() + (ag - bg).unsigned_abs() + (ab - bb).unsigned_abs()
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

/// Start the HP monitor. When `window_class` is non-empty, `x`/`y` are interpreted
/// as offsets within that window's client area and sampling reads from the window's
/// own DC (only when the window is in the foreground). When `window_class` is empty,
/// falls back to legacy absolute-screen-coord sampling for configs saved by older
/// versions; users should re-pick to migrate.
pub fn start(
    window_class: String,
    window_title: String,
    x: i32,
    y: i32,
    ref_color: u32,
) {
    if ACTIVE.load(Ordering::Acquire) {
        return;
    }

    CANCEL.store(false, Ordering::Release);
    ACTIVE.store(true, Ordering::Release);

    thread::spawn(move || {
        let use_window_anchor = !window_class.is_empty();
        if use_window_anchor {
            println!(
                "[Ranify2] HP monitor started (window=\"{}\" class=\"{}\" cx={} cy={} ref=0x{:06X})",
                window_title, window_class, x, y, ref_color
            );
        } else {
            println!(
                "[Ranify2] HP monitor started in legacy absolute-coord mode (x={}, y={}, ref=0x{:06X}). \
                 Re-pick the pixel in Settings to anchor it to the game window.",
                x, y, ref_color
            );
        }

        let vk: u16 = 0x51; // Q key
        let scan_code = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
        let class_w = if use_window_anchor {
            Some(wide(&window_class))
        } else {
            None
        };

        let mut at_reference = true;

        loop {
            if CANCEL.load(Ordering::Acquire) {
                break;
            }

            let color = unsafe {
                if use_window_anchor {
                    let class_ptr = class_w.as_ref().unwrap().as_ptr();
                    let hwnd = find_window_matching(class_ptr, &window_title);
                    if hwnd.is_null() {
                        thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                    if GetForegroundWindow() != hwnd {
                        thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    let hdc = GetDC(hwnd);
                    if hdc.is_null() {
                        thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    let c = GetPixel(hdc, x, y);
                    ReleaseDC(hwnd, hdc);
                    c
                } else {
                    let hdc = GetDC(std::ptr::null_mut());
                    if hdc.is_null() {
                        thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    let c = GetPixel(hdc, x, y);
                    ReleaseDC(std::ptr::null_mut(), hdc);
                    c
                }
            };

            if color != CLR_INVALID {
                let dist = color_dist(color, ref_color);
                let matches_ref = dist <= COLOR_TOLERANCE;

                if at_reference && !matches_ref {
                    println!(
                        "[Ranify2] HP threshold crossed (color=0x{:06X}, dist={}) -> Q",
                        color, dist
                    );
                    player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE);
                    player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);
                    at_reference = false;
                } else if !at_reference && matches_ref {
                    println!("[Ranify2] HP pixel re-armed (color=0x{:06X})", color);
                    at_reference = true;
                }
            }

            thread::sleep(Duration::from_millis(200));
        }

        println!("[Ranify2] HP monitor stopped.");
        ACTIVE.store(false, Ordering::Release);
    });
}

pub fn stop() {
    if ACTIVE.load(Ordering::Acquire) {
        CANCEL.store(true, Ordering::Release);
    }
}

pub(crate) unsafe fn find_window_matching(class_ptr: *const u16, title_prefix: &str) -> HWND {
    if title_prefix.is_empty() {
        return FindWindowW(class_ptr, std::ptr::null());
    }

    let mut hwnd = FindWindowExW(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        class_ptr,
        std::ptr::null(),
    );
    while !hwnd.is_null() {
        let mut buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) as usize;
        let actual: String = String::from_utf16_lossy(&buf[..len]);
        if actual.starts_with(title_prefix) {
            return hwnd;
        }
        hwnd = FindWindowExW(std::ptr::null_mut(), hwnd, class_ptr, std::ptr::null());
    }

    FindWindowW(class_ptr, std::ptr::null())
}
