use crate::win32_helpers::{wide, create_control};
use crate::{config, network, player, recorder};
use super::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

pub(crate) struct ToolbarControls {
    pub hwnd_main: HWND,
    pub hwnd_btn_record: HWND,
    pub hwnd_btn_play: HWND,
    pub hwnd_chk_loop: HWND,
    pub hwnd_chk_topmost: HWND,
    pub hwnd_status: HWND,
    pub config: config::AppConfig,
}

pub fn create_toolbar_window(cfg: &config::AppConfig) -> HWND {
    unsafe {
        let class_name = wide("RaniTaskToolbar");
        let hinstance = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(toolbar_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);

        // Calculate position: top-right of screen
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let win_w = 530;
        let win_h = 52;

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: win_w,
            bottom: win_h,
        };
        let style = WS_POPUP | WS_CAPTION | WS_SYSMENU;
        let ex_style = WS_EX_TOOLWINDOW
            | if cfg.always_on_top {
                WS_EX_TOPMOST
            } else {
                0
            };
        AdjustWindowRectEx(&mut rect, style, FALSE, ex_style as u32);

        let actual_w = rect.right - rect.left;
        let actual_h = rect.bottom - rect.top;
        let x = screen_w - actual_w - 10;
        let y = 10;

        let title = wide("RaniTask");
        let hwnd = CreateWindowExW(
            ex_style as u32,
            class_name.as_ptr(),
            title.as_ptr(),
            style | WS_VISIBLE,
            x,
            y,
            actual_w,
            actual_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        // Store config in controls struct
        let controls = Box::new(ToolbarControls {
            hwnd_main: hwnd,
            hwnd_btn_record: std::ptr::null_mut(),
            hwnd_btn_play: std::ptr::null_mut(),
            hwnd_chk_loop: std::ptr::null_mut(),
            hwnd_chk_topmost: std::ptr::null_mut(),
            hwnd_status: std::ptr::null_mut(),
            config: cfg.clone(),
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(controls) as isize);

        // Create child controls
        create_controls(hwnd, hinstance, cfg);

        // Start status timer
        SetTimer(hwnd, TIMER_STATUS, 200, None);

        hwnd
    }
}

unsafe fn create_controls(hwnd: HWND, hinstance: HINSTANCE, cfg: &config::AppConfig) {
    let font = GetStockObject(DEFAULT_GUI_FONT as i32) as HFONT;

    let y = 6;
    let h = 26;

    // -- Group 1: Recording controls --
    let btn_rec = create_control(
        hwnd, hinstance, font, "BUTTON", "Record",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
        6, y, 56, h, IDC_BTN_RECORD,
    );

    let btn_play = create_control(
        hwnd, hinstance, font, "BUTTON", "Play",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
        66, y, 50, h, IDC_BTN_PLAY,
    );

    // -- Group 2: Mode toggles (gap before) --
    let chk_loop = create_control(
        hwnd, hinstance, font, "BUTTON", "Loop",
        WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX as u32, 0,
        126, y, 54, h, IDC_CHK_LOOP,
    );
    if cfg.loop_playback {
        SendMessageW(chk_loop, BM_SETCHECK, BST_CHECKED as WPARAM, 0);
    }

    let chk_top = create_control(
        hwnd, hinstance, font, "BUTTON", "Top",
        WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX as u32, 0,
        182, y, 46, h, IDC_CHK_TOPMOST,
    );
    if cfg.always_on_top {
        SendMessageW(chk_top, BM_SETCHECK, BST_CHECKED as WPARAM, 0);
    }

    // -- Group 3: Navigation (gap before) --
    create_control(
        hwnd, hinstance, font, "BUTTON", "\u{2699}",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
        240, y, 26, h, IDC_BTN_SETTINGS,
    );

    create_control(
        hwnd, hinstance, font, "BUTTON", "Sequences",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
        270, y, 74, h, IDC_BTN_SEQUENCES,
    );

    create_control(
        hwnd, hinstance, font, "BUTTON", "Remote",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
        350, y, 66, h, IDC_BTN_REMOTE,
    );

    // -- Status label --
    let status = create_control(
        hwnd, hinstance, font, "STATIC", "Idle",
        WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
        424, y + 2, 96, h, IDC_STATUS,
    );

    // Update stored controls
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToolbarControls;
    if !ptr.is_null() {
        (*ptr).hwnd_btn_record = btn_rec;
        (*ptr).hwnd_btn_play = btn_play;
        (*ptr).hwnd_chk_loop = chk_loop;
        (*ptr).hwnd_chk_topmost = chk_top;
        (*ptr).hwnd_status = status;
    }
}

unsafe extern "system" fn toolbar_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let control_id = LOWORD(w_param as u32);
            match control_id {
                x if x == IDC_BTN_RECORD => handle_record_toggle(),
                x if x == IDC_BTN_PLAY => handle_play_toggle(),
                x if x == IDC_CHK_LOOP => {
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToolbarControls;
                    if !ptr.is_null() {
                        let checked = SendMessageW((*ptr).hwnd_chk_loop, BM_GETCHECK, 0, 0)
                            == BST_CHECKED as isize;
                        player::set_loop_mode(checked);
                        (*ptr).config.loop_playback = checked;
                        if let Err(e) = config::save_config(&(*ptr).config) {
                            eprintln!("[RaniTask] Config save failed: {}", e);
                        }
                    }
                }
                x if x == IDC_CHK_TOPMOST => {
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToolbarControls;
                    if !ptr.is_null() {
                        let checked = SendMessageW((*ptr).hwnd_chk_topmost, BM_GETCHECK, 0, 0)
                            == BST_CHECKED as isize;
                        let z_order = if checked {
                            HWND_TOPMOST
                        } else {
                            HWND_NOTOPMOST
                        };
                        SetWindowPos(hwnd, z_order, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                        (*ptr).config.always_on_top = checked;
                        if let Err(e) = config::save_config(&(*ptr).config) {
                            eprintln!("[RaniTask] Config save failed: {}", e);
                        }
                    }
                }
                x if x == IDC_BTN_SETTINGS => {
                    settings::show_settings_dialog(hwnd);
                }
                x if x == IDC_BTN_SEQUENCES => {
                    sequences::show_sequences_window(hwnd);
                }
                x if x == IDC_BTN_REMOTE => {
                    remote::show_remote_dialog(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_TIMER => {
            if w_param == TIMER_STATUS {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToolbarControls;
                if !ptr.is_null() {
                    // Update status text
                    let status = if recorder::is_recording() {
                        "Recording..."
                    } else if player::is_playing() {
                        if player::is_loop_mode() {
                            "Playing (loop)"
                        } else {
                            "Playing..."
                        }
                    } else if network::is_listening() {
                        "Idle (Recv)"
                    } else {
                        "Idle"
                    };
                    SetWindowTextW((*ptr).hwnd_status, wide(status).as_ptr());

                    // Update button text
                    let rec_text = if recorder::is_recording() {
                        "Stop"
                    } else {
                        "Record"
                    };
                    SetWindowTextW((*ptr).hwnd_btn_record, wide(rec_text).as_ptr());

                    let play_text = if player::is_playing() {
                        "Stop"
                    } else {
                        "Play"
                    };
                    SetWindowTextW((*ptr).hwnd_btn_play, wide(play_text).as_ptr());
                }
            }
            0
        }
        WM_CLOSE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToolbarControls;
            if !ptr.is_null() {
                let _ = Box::from_raw(ptr); // free the controls struct
            }
            KillTimer(hwnd, TIMER_STATUS);
            DestroyWindow(hwnd);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}
