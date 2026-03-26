use crate::player;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use winapi::um::wingdi::GetPixel;
use winapi::um::winuser::*;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

const CLR_INVALID: u32 = 0xFFFFFFFF;

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

pub fn start(x: i32, y: i32, ref_color: u32) {
    if ACTIVE.load(Ordering::Acquire) {
        return;
    }

    CANCEL.store(false, Ordering::Release);
    ACTIVE.store(true, Ordering::Release);

    thread::spawn(move || {
        println!(
            "[RaniTask] HP monitor started (x={}, y={}, ref=0x{:06X})",
            x, y, ref_color
        );

        let vk: u16 = 0x51; // Q key
        let scan_code = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;

        loop {
            if CANCEL.load(Ordering::Acquire) {
                break;
            }

            // Read pixel color at the configured coordinate
            let color = unsafe {
                let hdc = GetDC(std::ptr::null_mut());
                if hdc.is_null() {
                    thread::sleep(Duration::from_millis(200));
                    continue;
                }
                let c = GetPixel(hdc, x, y);
                ReleaseDC(std::ptr::null_mut(), hdc);
                c
            };

            if color != CLR_INVALID && color != ref_color {
                // HP dropped below threshold — press Q
                player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE);
                player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);
                // Cooldown after pressing potion key
                thread::sleep(Duration::from_millis(200));
            }

            thread::sleep(Duration::from_millis(200));
        }

        println!("[RaniTask] HP monitor stopped.");
        ACTIVE.store(false, Ordering::Release);
    });
}

pub fn stop() {
    if ACTIVE.load(Ordering::Acquire) {
        CANCEL.store(true, Ordering::Release);
    }
}
