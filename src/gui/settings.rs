use crate::win32_helpers::{wide, create_control, register_and_create_dialog, populate_key_combo, KEY_OPTIONS};
use crate::{config, hotkeys};
use super::*;
use super::toolbar::ToolbarControls;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

static SETTINGS_HWND: AtomicIsize = AtomicIsize::new(0);
static SAMPLED_COLOR: AtomicU32 = AtomicU32::new(0);
static PICKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub unsafe fn show_settings_dialog(parent: HWND) {
    let existing = SETTINGS_HWND.load(Ordering::Acquire) as HWND;
    if !existing.is_null() && IsWindow(existing) != 0 {
        SetForegroundWindow(existing);
        return;
    }

    let hinstance = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());

    // Position near the parent toolbar
    let mut parent_rect: RECT = std::mem::zeroed();
    GetWindowRect(parent, &mut parent_rect);

    let sx = parent_rect.left;
    let sy = parent_rect.bottom + 4;

    let hwnd = register_and_create_dialog(
        "RaniTaskSettings", "Settings",
        settings_wnd_proc,
        WS_EX_TOOLWINDOW as u32,
        WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        sx, sy, 300, 340,
        parent, hinstance,
    );
    SETTINGS_HWND.store(hwnd as isize, Ordering::Release);
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let hinstance = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());
            let font = GetStockObject(DEFAULT_GUI_FONT as i32) as HFONT;

            // "Record Key:" label
            create_control(
                hwnd, hinstance, font, "STATIC", "Record Key:",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                12, 16, 80, 20, 0,
            );

            // Record key combobox
            create_control(
                hwnd, hinstance, font, "COMBOBOX", "",
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST as u32 | WS_VSCROLL, 0,
                96, 12, 148, 200, IDC_COMBO_RECORD_KEY,
            );

            // "Stop Key:" label
            create_control(
                hwnd, hinstance, font, "STATIC", "Stop Key:",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                12, 52, 80, 20, 0,
            );

            // Stop key combobox
            create_control(
                hwnd, hinstance, font, "COMBOBOX", "",
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST as u32 | WS_VSCROLL, 0,
                96, 48, 148, 200, IDC_COMBO_PLAY_KEY,
            );

            // Populate comboboxes
            let (current_rec, current_stop) = hotkeys::current_hotkeys();
            let h_combo_rec = GetDlgItem(hwnd, IDC_COMBO_RECORD_KEY as i32);
            let h_combo_play = GetDlgItem(hwnd, IDC_COMBO_PLAY_KEY as i32);
            populate_key_combo(h_combo_rec, KEY_OPTIONS, Some(current_rec));
            populate_key_combo(h_combo_play, KEY_OPTIONS, Some(current_stop));

            // "Queue Key:" label
            create_control(
                hwnd, hinstance, font, "STATIC", "Queue Key:",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                12, 88, 80, 20, 0,
            );

            // Queue key combobox
            create_control(
                hwnd, hinstance, font, "COMBOBOX", "",
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST as u32 | WS_VSCROLL, 0,
                96, 84, 148, 200, IDC_COMBO_QUEUE_KEY,
            );

            // Populate queue key combo with "(None)" option
            let h_combo_queue = GetDlgItem(hwnd, IDC_COMBO_QUEUE_KEY as i32);
            let none_text = wide("(None)");
            SendMessageW(h_combo_queue, CB_ADDSTRING, 0, none_text.as_ptr() as LPARAM);

            let current_queue_vk = hotkeys::current_queue_vk();
            if current_queue_vk.is_none() {
                SendMessageW(h_combo_queue, CB_SETCURSEL, 0, 0);
            }
            for (i, (vk, name)) in KEY_OPTIONS.iter().enumerate() {
                let wname = wide(name);
                SendMessageW(h_combo_queue, CB_ADDSTRING, 0, wname.as_ptr() as LPARAM);
                if current_queue_vk == Some(*vk) {
                    SendMessageW(h_combo_queue, CB_SETCURSEL, (i + 1) as WPARAM, 0);
                }
            }

            // -- HP Monitor section --
            create_control(
                hwnd, hinstance, font, "STATIC", "— HP Monitor —",
                WS_CHILD | WS_VISIBLE | SS_CENTER, 0,
                12, 124, 272, 18, 0,
            );

            // X coordinate
            create_control(
                hwnd, hinstance, font, "STATIC", "X:",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                12, 150, 16, 20, 0,
            );
            create_control(
                hwnd, hinstance, font, "EDIT", "",
                WS_CHILD | WS_VISIBLE | WS_BORDER | ES_NUMBER as u32, 0,
                30, 148, 70, 22, IDC_EDIT_HP_X,
            );

            // Y coordinate
            create_control(
                hwnd, hinstance, font, "STATIC", "Y:",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                112, 150, 16, 20, 0,
            );
            create_control(
                hwnd, hinstance, font, "EDIT", "",
                WS_CHILD | WS_VISIBLE | WS_BORDER | ES_NUMBER as u32, 0,
                130, 148, 70, 22, IDC_EDIT_HP_Y,
            );

            // Pick button — click to enter screen pixel picker mode
            create_control(
                hwnd, hinstance, font, "BUTTON", "Pick",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                210, 148, 60, 22, IDC_BTN_HP_PICK,
            );

            // Sample button (manual)
            create_control(
                hwnd, hinstance, font, "BUTTON", "Sample",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                12, 180, 60, 24, IDC_BTN_HP_SAMPLE,
            );

            // Color preview label
            create_control(
                hwnd, hinstance, font, "STATIC", "(not sampled)",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                80, 183, 190, 20, IDC_STATIC_HP_COLOR,
            );

            // Live picker display — shows cursor X, Y and color while picking
            create_control(
                hwnd, hinstance, font, "STATIC", "",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                12, 210, 268, 20, IDC_STATIC_HP_LIVE,
            );

            // Pre-populate HP fields from config
            let parent = GetParent(hwnd);
            let parent_ptr = GetWindowLongPtrW(parent, GWLP_USERDATA) as *mut ToolbarControls;
            if !parent_ptr.is_null() {
                let cfg = &(*parent_ptr).config;
                if cfg.hp_monitor_x != 0 || cfg.hp_monitor_y != 0 {
                    let x_text = wide(&cfg.hp_monitor_x.to_string());
                    let y_text = wide(&cfg.hp_monitor_y.to_string());
                    SetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_X as i32), x_text.as_ptr());
                    SetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_Y as i32), y_text.as_ptr());
                }
                if cfg.hp_monitor_color != 0 {
                    SAMPLED_COLOR.store(cfg.hp_monitor_color, Ordering::Release);
                    let r = cfg.hp_monitor_color & 0xFF;
                    let g = (cfg.hp_monitor_color >> 8) & 0xFF;
                    let b = (cfg.hp_monitor_color >> 16) & 0xFF;
                    let color_text = wide(&format!("R:{} G:{} B:{}", r, g, b));
                    SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_COLOR as i32), color_text.as_ptr());
                } else {
                    SAMPLED_COLOR.store(0, Ordering::Release);
                }
            }

            // OK button
            create_control(
                hwnd, hinstance, font, "BUTTON", "OK",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                115, 252, 70, 28, IDC_BTN_SETTINGS_OK,
            );

            0
        }
        WM_COMMAND => {
            let control_id = LOWORD(w_param as u32);
            if control_id == IDC_BTN_HP_PICK {
                // Toggle pick mode
                if PICKING.load(Ordering::Acquire) {
                    // Stop picking
                    PICKING.store(false, Ordering::Release);
                    KillTimer(hwnd, TIMER_HP_PICK);
                    let text = wide("Pick");
                    SetWindowTextW(GetDlgItem(hwnd, IDC_BTN_HP_PICK as i32), text.as_ptr());
                    let text = wide("");
                    SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_LIVE as i32), text.as_ptr());
                } else {
                    // Start picking — poll cursor every 50ms
                    PICKING.store(true, Ordering::Release);
                    SetTimer(hwnd, TIMER_HP_PICK, 50, None);
                    let text = wide("Stop");
                    SetWindowTextW(GetDlgItem(hwnd, IDC_BTN_HP_PICK as i32), text.as_ptr());
                    let text = wide("Move to HP pixel, press INSERT");
                    SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_LIVE as i32), text.as_ptr());
                }
                return 0;
            } else if control_id == IDC_BTN_HP_SAMPLE {
                // Read X, Y from edit boxes and sample pixel color
                let mut buf_x = [0u16; 16];
                let mut buf_y = [0u16; 16];
                GetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_X as i32), buf_x.as_mut_ptr(), 16);
                GetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_Y as i32), buf_y.as_mut_ptr(), 16);

                let x_str: String = buf_x.iter().take_while(|&&c| c != 0).map(|&c| c as u8 as char).collect();
                let y_str: String = buf_y.iter().take_while(|&&c| c != 0).map(|&c| c as u8 as char).collect();

                if let (Ok(x), Ok(y)) = (x_str.parse::<i32>(), y_str.parse::<i32>()) {
                    let hdc = GetDC(std::ptr::null_mut());
                    if !hdc.is_null() {
                        let color = GetPixel(hdc, x, y);
                        ReleaseDC(std::ptr::null_mut(), hdc);

                        if color != 0xFFFFFFFF {
                            SAMPLED_COLOR.store(color, Ordering::Release);
                            let r = color & 0xFF;
                            let g = (color >> 8) & 0xFF;
                            let b = (color >> 16) & 0xFF;
                            let text = wide(&format!("R:{} G:{} B:{}", r, g, b));
                            SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_COLOR as i32), text.as_ptr());
                        } else {
                            let text = wide("(invalid pixel)");
                            SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_COLOR as i32), text.as_ptr());
                        }
                    }
                } else {
                    let msg = wide("Enter valid X and Y coordinates.");
                    let title = wide("HP Monitor");
                    MessageBoxW(hwnd, msg.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING);
                }
                return 0;
            } else if control_id == IDC_BTN_SETTINGS_OK {
                // Read selections
                let h_combo_rec = GetDlgItem(hwnd, IDC_COMBO_RECORD_KEY as i32);
                let h_combo_play = GetDlgItem(hwnd, IDC_COMBO_PLAY_KEY as i32);

                let rec_idx = SendMessageW(h_combo_rec, CB_GETCURSEL, 0, 0) as usize;
                let play_idx = SendMessageW(h_combo_play, CB_GETCURSEL, 0, 0) as usize;

                if rec_idx < KEY_OPTIONS.len() && play_idx < KEY_OPTIONS.len() {
                    let new_rec_vk = KEY_OPTIONS[rec_idx].0;
                    let new_stop_vk = KEY_OPTIONS[play_idx].0;

                    // Read queue key selection
                    let h_combo_queue = GetDlgItem(hwnd, IDC_COMBO_QUEUE_KEY as i32);
                    let queue_idx = SendMessageW(h_combo_queue, CB_GETCURSEL, 0, 0) as usize;
                    let new_queue_vk: Option<u16> = if queue_idx == 0 {
                        None // "(None)" selected
                    } else if queue_idx - 1 < KEY_OPTIONS.len() {
                        Some(KEY_OPTIONS[queue_idx - 1].0)
                    } else {
                        None
                    };

                    if new_rec_vk == new_stop_vk {
                        let msg = wide("Record and Stop keys must be different!");
                        let title = wide("Error");
                        MessageBoxW(
                            hwnd,
                            msg.as_ptr(),
                            title.as_ptr(),
                            MB_OK | MB_ICONERROR,
                        );
                        return 0;
                    }

                    if let Some(qvk) = new_queue_vk {
                        if qvk == new_rec_vk || qvk == new_stop_vk {
                            let msg = wide("Queue key must be different from Record and Stop keys!");
                            let title = wide("Error");
                            MessageBoxW(
                                hwnd,
                                msg.as_ptr(),
                                title.as_ptr(),
                                MB_OK | MB_ICONERROR,
                            );
                            return 0;
                        }
                    }

                    if hotkeys::reregister_hotkeys(new_rec_vk, new_stop_vk) {
                        hotkeys::set_queue_vk(new_queue_vk);
                        let parent = GetParent(hwnd);
                        let ptr =
                            GetWindowLongPtrW(parent, GWLP_USERDATA) as *mut ToolbarControls;
                        if !ptr.is_null() {
                            (*ptr).config.record_vk = new_rec_vk;
                            (*ptr).config.stop_vk = new_stop_vk;
                            (*ptr).config.queue_vk = new_queue_vk;

                            // Save HP monitor settings
                            let mut buf_x = [0u16; 16];
                            let mut buf_y = [0u16; 16];
                            GetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_X as i32), buf_x.as_mut_ptr(), 16);
                            GetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_Y as i32), buf_y.as_mut_ptr(), 16);
                            let x_str: String = buf_x.iter().take_while(|&&c| c != 0).map(|&c| c as u8 as char).collect();
                            let y_str: String = buf_y.iter().take_while(|&&c| c != 0).map(|&c| c as u8 as char).collect();
                            (*ptr).config.hp_monitor_x = x_str.parse().unwrap_or(0);
                            (*ptr).config.hp_monitor_y = y_str.parse().unwrap_or(0);
                            (*ptr).config.hp_monitor_color = SAMPLED_COLOR.load(Ordering::Acquire);

                            if let Err(e) = config::save_config(&(*ptr).config) {
                                eprintln!("[RaniTask] Config save failed: {}", e);
                            }
                        }
                        DestroyWindow(hwnd);
                        SETTINGS_HWND.store(0, Ordering::Release);
                    } else {
                        let msg = wide("Failed to register hotkeys.\nKey may be in use.");
                        let title = wide("Error");
                        MessageBoxW(
                            hwnd,
                            msg.as_ptr(),
                            title.as_ptr(),
                            MB_OK | MB_ICONERROR,
                        );
                    }
                }
            }
            0
        }
        WM_TIMER => {
            if w_param == TIMER_HP_PICK && PICKING.load(Ordering::Acquire) {
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);

                // Read pixel color at cursor
                let hdc = GetDC(std::ptr::null_mut());
                let color = if !hdc.is_null() {
                    let c = GetPixel(hdc, pt.x, pt.y);
                    ReleaseDC(std::ptr::null_mut(), hdc);
                    c
                } else {
                    0xFFFFFFFF
                };

                // Update live display
                if color != 0xFFFFFFFF {
                    let r = color & 0xFF;
                    let g = (color >> 8) & 0xFF;
                    let b = (color >> 16) & 0xFF;
                    let text = wide(&format!(
                        "X:{} Y:{} | R:{} G:{} B:{}",
                        pt.x, pt.y, r, g, b
                    ));
                    SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_LIVE as i32), text.as_ptr());
                } else {
                    let text = wide(&format!("X:{} Y:{} | (unreadable)", pt.x, pt.y));
                    SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_LIVE as i32), text.as_ptr());
                }

                // Check if INSERT key is pressed to capture
                let insert_down = GetAsyncKeyState(VK_INSERT) & (0x8000u16 as i16) != 0;
                if insert_down {
                    {
                        // Captured! Fill X/Y fields and sample color
                        let x_text = wide(&pt.x.to_string());
                        let y_text = wide(&pt.y.to_string());
                        SetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_X as i32), x_text.as_ptr());
                        SetWindowTextW(GetDlgItem(hwnd, IDC_EDIT_HP_Y as i32), y_text.as_ptr());

                        if color != 0xFFFFFFFF {
                            SAMPLED_COLOR.store(color, Ordering::Release);
                            let r = color & 0xFF;
                            let g = (color >> 8) & 0xFF;
                            let b = (color >> 16) & 0xFF;
                            let text = wide(&format!("R:{} G:{} B:{}", r, g, b));
                            SetWindowTextW(
                                GetDlgItem(hwnd, IDC_STATIC_HP_COLOR as i32),
                                text.as_ptr(),
                            );
                        }

                        // Stop pick mode
                        PICKING.store(false, Ordering::Release);
                        KillTimer(hwnd, TIMER_HP_PICK);
                        let text = wide("Pick");
                        SetWindowTextW(GetDlgItem(hwnd, IDC_BTN_HP_PICK as i32), text.as_ptr());
                        let text = wide("Pixel captured!");
                        SetWindowTextW(GetDlgItem(hwnd, IDC_STATIC_HP_LIVE as i32), text.as_ptr());
                    }
                }
            }
            0
        }
        WM_CLOSE => {
            if PICKING.load(Ordering::Acquire) {
                PICKING.store(false, Ordering::Release);
                KillTimer(hwnd, TIMER_HP_PICK);
            }
            DestroyWindow(hwnd);
            SETTINGS_HWND.store(0, Ordering::Release);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}
