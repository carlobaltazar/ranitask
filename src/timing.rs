use winapi::um::profileapi::{QueryPerformanceCounter, QueryPerformanceFrequency};
use winapi::shared::ntdef::LARGE_INTEGER;

const SPIN_WAIT_THRESHOLD_MICROS: i64 = 2000;
const SPIN_WAIT_MARGIN_MICROS: i64 = 1500;

pub struct PrecisionTimer {
    frequency: i64,
}

impl PrecisionTimer {
    pub fn new() -> Self {
        let mut freq: LARGE_INTEGER = unsafe { std::mem::zeroed() };
        unsafe {
            QueryPerformanceFrequency(&mut freq);
        }
        PrecisionTimer {
            frequency: unsafe { *freq.QuadPart() },
        }
    }

    pub fn now_ticks(&self) -> i64 {
        let mut ticks: LARGE_INTEGER = unsafe { std::mem::zeroed() };
        unsafe {
            QueryPerformanceCounter(&mut ticks);
            *ticks.QuadPart()
        }
    }

    pub fn ticks_to_micros(&self, ticks: i64) -> i64 {
        (ticks * 1_000_000) / self.frequency
    }

    /// Hybrid wait: sleep for bulk of the time, then spin-wait for precision.
    pub fn precise_wait_micros(&self, micros: i64) {
        if micros <= 0 {
            return;
        }

        let start = self.now_ticks();
        let target_ticks = (micros * self.frequency) / 1_000_000;

        // For delays > 2ms, sleep for most of it to avoid burning CPU
        if micros > SPIN_WAIT_THRESHOLD_MICROS {
            let sleep_ms = ((micros - SPIN_WAIT_MARGIN_MICROS) / 1000) as u64;
            if sleep_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
            }
        }

        // Spin-wait for the remainder (sub-millisecond precision)
        loop {
            let elapsed = self.now_ticks() - start;
            if elapsed >= target_ticks {
                break;
            }
            std::hint::spin_loop();
        }
    }
}
