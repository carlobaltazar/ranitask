#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod gui;
mod hotkeys;
mod hp_monitor;
mod network;
mod pet_cycle;
mod player;
mod recorder;
mod sequence;
mod storage;
mod timing;
mod win32_helpers;

use winapi::um::winuser::*;

fn main() {
    // Load config
    let cfg = config::load_config();

    // Set loop mode and shuffle mode from config
    player::set_loop_mode(cfg.loop_playback);
    player::set_shuffle_mode(cfg.shuffle_queue);

    // Create GUI toolbar window
    let hwnd = gui::create_toolbar_window(&cfg);
    if hwnd.is_null() {
        eprintln!("[RaniTask] Failed to create window.");
        return;
    }

    // Install global low-level keyboard hook for hotkeys
    // (works even when fullscreen games have focus, unlike RegisterHotKey)
    if !hotkeys::install_hook(cfg.record_vk, cfg.stop_vk) {
        unsafe {
            let msg = win32_helpers::wide("Failed to install hotkey hook.");
            let title = win32_helpers::wide("RaniTask Error");
            MessageBoxW(
                std::ptr::null_mut(),
                msg.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
        return;
    }

    // Load all sequence hotkey bindings at startup
    let bindings = gui::load_all_bindings();
    hotkeys::set_sequence_bindings(bindings);

    // Load remote hotkey bindings
    let remote_binds: Vec<(u32, u16, String)> = cfg.remote_bindings
        .iter()
        .map(|b| (b.modifiers, b.vk_code, b.sequence_name.clone()))
        .collect();
    hotkeys::set_remote_bindings(remote_binds);

    // Load queue hotkey binding
    if let Some(qvk) = cfg.queue_vk {
        hotkeys::set_queue_vk(Some(qvk));
    }

    // Auto-start receiver if configured
    if cfg.remote_auto_listen && cfg.remote_port > 0 {
        let password = if cfg.remote_password.is_empty() {
            None
        } else {
            Some(cfg.remote_password.clone())
        };
        if let Err(e) = network::start_listener(cfg.remote_port, password) {
            eprintln!("[RaniTask] Auto-listen failed: {}", e);
        }
    }

    // Auto-start pet cycle if configured
    if cfg.pet_cycle_enabled {
        pet_cycle::start(cfg.pet_cycle_interval_secs);
    }

    // Auto-start HP monitor if configured
    if cfg.hp_monitor_enabled && cfg.hp_monitor_color != 0 {
        hp_monitor::start(cfg.hp_monitor_x, cfg.hp_monitor_y, cfg.hp_monitor_color);
    }

    // Load default sequence (or fall back to last saved) into LAST_EVENTS
    let load_name = cfg.default_sequence.clone()
        .or_else(|| storage::list_sequences().ok().and_then(|n| n.last().cloned()));
    if let Some(name) = load_name {
        if let Ok(seq) = storage::load_sequence(&name) {
            *win32_helpers::lock_or_recover(&gui::LAST_EVENTS) = Some(seq.events);
        }
    }

    // Win32 message loop
    let mut msg = unsafe { std::mem::zeroed::<MSG>() };
    loop {
        let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
        if ret <= 0 {
            break;
        }

        // Handle hotkey messages from our low-level hook
        if msg.message == hotkeys::WM_APP_HOTKEY {
            let hotkey_id = msg.wParam as i32;
            match hotkey_id {
                hotkeys::HOTKEY_TOGGLE_RECORD => gui::handle_record_toggle(),
                hotkeys::HOTKEY_PLAY_STOP => gui::handle_play_toggle(),
                hotkeys::HOTKEY_PLAY_QUEUE => gui::handle_play_queue_hotkey(),
                hotkeys::HOTKEY_PLAY_SEQUENCE => {
                    let vk = msg.lParam as u16;
                    gui::handle_play_sequence(vk);
                }
                hotkeys::HOTKEY_REMOTE_SEND => {
                    let index = msg.lParam as usize;
                    if let Some(seq_name) = hotkeys::remote_binding_at(index) {
                        let cfg = config::load_config();
                        if !cfg.remote_hosts.is_empty() {
                            let port = cfg.remote_port;
                            let pw = if cfg.remote_password.is_empty() {
                                None
                            } else {
                                Some(cfg.remote_password)
                            };
                            let cmd = format!("PLAY {}", seq_name);
                            for host in cfg.remote_hosts {
                                let cmd = cmd.clone();
                                let pw = pw.clone();
                                std::thread::spawn(move || {
                                    if let Err(e) = network::send_command(&host, port, pw.as_deref(), &cmd) {
                                        eprintln!("[RaniTask] Remote send to {} failed: {}", host, e);
                                    }
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
            continue;
        }

        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    // Cleanup
    hp_monitor::stop();
    pet_cycle::stop();
    network::stop_listener();
    hotkeys::uninstall_hook();
}
