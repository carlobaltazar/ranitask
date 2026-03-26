use crate::player;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use winapi::um::winuser::*;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

pub fn start(interval_secs: u64) {
    if ACTIVE.load(Ordering::Acquire) {
        return; // already running
    }

    CANCEL.store(false, Ordering::Release);
    ACTIVE.store(true, Ordering::Release);

    thread::spawn(move || {
        println!("[RaniTask] Pet cycle started (interval: {}s)", interval_secs);

        // Get scan code for 'A' key (VK 0x41)
        let vk: u16 = 0x41;
        let scan_code = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;

        loop {
            // Sleep in 1-second chunks so we can respond to cancel quickly
            for _ in 0..interval_secs {
                if CANCEL.load(Ordering::Acquire) {
                    println!("[RaniTask] Pet cycle stopped.");
                    ACTIVE.store(false, Ordering::Release);
                    return;
                }
                thread::sleep(Duration::from_secs(1));
            }

            // Check cancel one more time before sending keys
            if CANCEL.load(Ordering::Acquire) {
                break;
            }

            // Press "A" (hide pet)
            player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE);
            player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);

            // Wait 300ms
            thread::sleep(Duration::from_millis(300));

            // Press "A" again (call pet)
            player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE);
            player::send_key_input(vk, scan_code, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);

            println!("[RaniTask] Pet cycle: hide/call sent.");
        }

        println!("[RaniTask] Pet cycle stopped.");
        ACTIVE.store(false, Ordering::Release);
    });
}

pub fn stop() {
    if ACTIVE.load(Ordering::Acquire) {
        CANCEL.store(true, Ordering::Release);
    }
}
