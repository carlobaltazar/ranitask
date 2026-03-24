use crate::win32_helpers::{wide, create_control, register_and_create_dialog, lock_or_recover, vk_name};
use crate::{config, hotkeys, player, recorder, storage};
use super::*;
use std::sync::atomic::{AtomicIsize, Ordering};
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

static SEQUENCES_HWND: AtomicIsize = AtomicIsize::new(0);

pub unsafe fn show_sequences_window(parent: HWND) {
    let existing = SEQUENCES_HWND.load(Ordering::Acquire) as HWND;
    if !existing.is_null() && IsWindow(existing) != 0 {
        SetForegroundWindow(existing);
        return;
    }

    let hinstance = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());

    // Position near parent
    let mut parent_rect: RECT = std::mem::zeroed();
    GetWindowRect(parent, &mut parent_rect);

    let sx = parent_rect.left;
    let sy = parent_rect.bottom + 4;

    let hwnd = register_and_create_dialog(
        "RaniTaskSequences", "Sequences",
        sequences_wnd_proc,
        WS_EX_TOOLWINDOW as u32,
        WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        sx, sy, 560, 420,
        parent, hinstance,
    );
    SEQUENCES_HWND.store(hwnd as isize, Ordering::Release);
}

/// Refresh the sequences list if the window is open.
pub unsafe fn refresh_sequences_list() {
    let hwnd = SEQUENCES_HWND.load(Ordering::Acquire) as HWND;
    if !hwnd.is_null() && IsWindow(hwnd) != 0 {
        let h_list = GetDlgItem(hwnd, IDC_LIST_SEQUENCES as i32);
        if !h_list.is_null() {
            SendMessageW(h_list, LB_RESETCONTENT, 0, 0);
            populate_sequences_list(h_list);
        }
    }
}

unsafe extern "system" fn sequences_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let hinstance = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());
            let font = GetStockObject(DEFAULT_GUI_FONT as i32) as HFONT;

            // -- Left panel: Saved Sequences --
            create_control(
                hwnd, hinstance, font, "STATIC", "Saved Sequences",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                10, 2, 200, 16, 0,
            );

            let h_list = create_control(
                hwnd, hinstance, font, "LISTBOX", "",
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | LBS_NOTIFY as u32,
                WS_EX_CLIENTEDGE as u32,
                10, 20, 220, 290, IDC_LIST_SEQUENCES,
            );
            populate_sequences_list(h_list);

            // -- Transfer buttons (between panels) --
            create_control(
                hwnd, hinstance, font, "BUTTON", ">>",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                237, 105, 42, 30, IDC_BTN_QUEUE_ADD,
            );

            create_control(
                hwnd, hinstance, font, "BUTTON", "<<",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                237, 145, 42, 30, IDC_BTN_QUEUE_REMOVE,
            );

            // -- Right panel: Queue --
            create_control(
                hwnd, hinstance, font, "STATIC", "Queue",
                WS_CHILD | WS_VISIBLE | SS_LEFT, 0,
                286, 2, 200, 16, 0,
            );

            create_control(
                hwnd, hinstance, font, "LISTBOX", "",
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | LBS_NOTIFY as u32,
                WS_EX_CLIENTEDGE as u32,
                286, 20, 220, 290, IDC_LIST_QUEUE,
            );
            refresh_queue_list(hwnd);

            // Queue reorder buttons
            create_control(
                hwnd, hinstance, font, "BUTTON", "Up",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                286, 314, 55, 26, IDC_BTN_QUEUE_UP,
            );

            create_control(
                hwnd, hinstance, font, "BUTTON", "Down",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                346, 314, 55, 26, IDC_BTN_QUEUE_DOWN,
            );

            // -- Bottom row: action buttons --
            create_control(
                hwnd, hinstance, font, "BUTTON", "Bind Key",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                10, 350, 80, 30, IDC_BTN_BIND_KEY,
            );

            create_control(
                hwnd, hinstance, font, "BUTTON", "Delete",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                96, 350, 64, 30, IDC_BTN_DELETE_SEQ,
            );

            create_control(
                hwnd, hinstance, font, "BUTTON", "Play",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                166, 350, 44, 30, IDC_BTN_PLAY_SEQ,
            );

            create_control(
                hwnd, hinstance, font, "BUTTON", "Default",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                214, 350, 54, 30, IDC_BTN_SET_DEFAULT,
            );

            // Shuffle checkbox
            let chk_shuffle = create_control(
                hwnd, hinstance, font, "BUTTON", "Shuffle",
                WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX as u32, 0,
                286, 350, 70, 30, IDC_CHK_SHUFFLE,
            );
            if player::is_shuffle_mode() {
                SendMessageW(chk_shuffle, BM_SETCHECK, BST_CHECKED as WPARAM, 0);
            }

            create_control(
                hwnd, hinstance, font, "BUTTON", "Play Queue",
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32, 0,
                400, 350, 104, 30, IDC_BTN_PLAY_QUEUE,
            );

            0
        }
        WM_COMMAND => {
            let control_id = LOWORD(w_param as u32);
            match control_id {
                x if x == IDC_BTN_BIND_KEY => handle_bind_key(hwnd),
                x if x == IDC_BTN_DELETE_SEQ => handle_delete_seq(hwnd),
                x if x == IDC_BTN_PLAY_SEQ => handle_play_seq(hwnd),
                x if x == IDC_BTN_QUEUE_ADD => handle_queue_add(hwnd),
                x if x == IDC_BTN_QUEUE_REMOVE => handle_queue_remove(hwnd),
                x if x == IDC_BTN_QUEUE_UP => handle_queue_up(hwnd),
                x if x == IDC_BTN_QUEUE_DOWN => handle_queue_down(hwnd),
                x if x == IDC_BTN_SET_DEFAULT => handle_set_default(hwnd),
                x if x == IDC_BTN_PLAY_QUEUE => handle_play_queue(),
                x if x == IDC_CHK_SHUFFLE => {
                    let h_chk = GetDlgItem(hwnd, IDC_CHK_SHUFFLE as i32);
                    let checked = SendMessageW(h_chk, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
                    player::set_shuffle_mode(checked);
                    let mut cfg = config::load_config();
                    cfg.shuffle_queue = checked;
                    let _ = config::save_config(&cfg);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            SEQUENCES_HWND.store(0, Ordering::Release);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

// ---- Command handlers ----

unsafe fn handle_bind_key(hwnd: HWND) {
    if let Some(name) = get_selected_sequence_name(hwnd) {
        *lock_or_recover(&BIND_SEQ_NAME) = Some(name);
        bind_key::show_bind_key_dialog(hwnd);
    }
}

unsafe fn handle_delete_seq(hwnd: HWND) {
    if let Some(name) = get_selected_sequence_name(hwnd) {
        let msg = wide(&format!("Delete \"{}\"?", name));
        let title = wide("Confirm Delete");
        let result = MessageBoxW(
            hwnd,
            msg.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_ICONQUESTION,
        );
        if result == IDYES {
            if let Err(e) = storage::delete_sequence(&name) {
                eprintln!("[RaniTask] Failed to delete sequence: {}", e);
            }
            refresh_bindings();
            // Clear default sequence if it was the deleted one
            let mut cfg = config::load_config();
            if cfg.default_sequence.as_deref() == Some(&name) {
                cfg.default_sequence = None;
                let _ = config::save_config(&cfg);
            }
            lock_or_recover(&SEQUENCE_QUEUE).retain(|n| n != &name);
            let h_list = GetDlgItem(hwnd, IDC_LIST_SEQUENCES as i32);
            SendMessageW(h_list, LB_RESETCONTENT, 0, 0);
            populate_sequences_list(h_list);
            refresh_queue_list(hwnd);
        }
    }
}

unsafe fn handle_set_default(hwnd: HWND) {
    if let Some(name) = get_selected_sequence_name(hwnd) {
        // Save as default sequence in config
        let mut cfg = config::load_config();
        cfg.default_sequence = Some(name.clone());
        let _ = config::save_config(&cfg);

        // Also load into LAST_EVENTS for immediate F11 use
        if let Ok(seq) = storage::load_sequence(&name) {
            *lock_or_recover(&LAST_EVENTS) = Some(seq.events);
        }
    }
}

unsafe fn handle_play_seq(hwnd: HWND) {
    if let Some(name) = get_selected_sequence_name(hwnd) {
        if !player::is_playing() && !recorder::is_recording() {
            if let Ok(seq) = storage::load_sequence(&name) {
                *lock_or_recover(&LAST_EVENTS) = Some(seq.events.clone());
                player::play_sequence(seq.events);
            }
        }
    }
}

unsafe fn handle_queue_add(hwnd: HWND) {
    if let Some(name) = get_selected_sequence_name(hwnd) {
        lock_or_recover(&SEQUENCE_QUEUE).push(name);
        refresh_queue_list(hwnd);
    }
}

unsafe fn handle_queue_remove(hwnd: HWND) {
    let h_queue = GetDlgItem(hwnd, IDC_LIST_QUEUE as i32);
    let idx = SendMessageW(h_queue, LB_GETCURSEL, 0, 0);
    if idx >= 0 {
        let idx = idx as usize;
        let mut queue = lock_or_recover(&SEQUENCE_QUEUE);
        if idx < queue.len() {
            queue.remove(idx);
        }
        drop(queue);
        refresh_queue_list(hwnd);
    }
}

unsafe fn handle_queue_up(hwnd: HWND) {
    let h_queue = GetDlgItem(hwnd, IDC_LIST_QUEUE as i32);
    let idx = SendMessageW(h_queue, LB_GETCURSEL, 0, 0);
    if idx > 0 {
        let idx = idx as usize;
        let mut queue = lock_or_recover(&SEQUENCE_QUEUE);
        if idx < queue.len() {
            queue.swap(idx, idx - 1);
        }
        drop(queue);
        refresh_queue_list(hwnd);
        SendMessageW(h_queue, LB_SETCURSEL, (idx - 1) as WPARAM, 0);
    }
}

unsafe fn handle_queue_down(hwnd: HWND) {
    let h_queue = GetDlgItem(hwnd, IDC_LIST_QUEUE as i32);
    let idx = SendMessageW(h_queue, LB_GETCURSEL, 0, 0);
    if idx >= 0 {
        let idx = idx as usize;
        let mut queue = lock_or_recover(&SEQUENCE_QUEUE);
        if idx + 1 < queue.len() {
            queue.swap(idx, idx + 1);
            drop(queue);
            refresh_queue_list(hwnd);
            SendMessageW(h_queue, LB_SETCURSEL, (idx + 1) as WPARAM, 0);
        }
    }
}

unsafe fn handle_play_queue() {
    if player::is_playing() || recorder::is_recording() {
        return;
    }
    let queue = lock_or_recover(&SEQUENCE_QUEUE).clone();
    if queue.is_empty() {
        return;
    }
    let mut event_lists = Vec::new();
    for name in &queue {
        if let Ok(seq) = storage::load_sequence(name) {
            event_lists.push(seq.events);
        }
    }
    if !event_lists.is_empty() {
        player::play_queue(event_lists);
    }
}

// ---- Helpers ----

/// Get the raw sequence name from the selected listbox item (strips the [key] suffix).
unsafe fn get_selected_sequence_name(hwnd: HWND) -> Option<String> {
    let h_list = GetDlgItem(hwnd, IDC_LIST_SEQUENCES as i32);
    let idx = SendMessageW(h_list, LB_GETCURSEL, 0, 0);
    if idx < 0 {
        return None;
    }
    let len = SendMessageW(h_list, LB_GETTEXTLEN, idx as WPARAM, 0);
    if len <= 0 {
        return None;
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    SendMessageW(h_list, LB_GETTEXT, idx as WPARAM, buf.as_mut_ptr() as LPARAM);
    let display = String::from_utf16_lossy(&buf[..len as usize]);
    // Strip " [Fx]" suffix if present
    if let Some(bracket_pos) = display.rfind(" [") {
        Some(display[..bracket_pos].to_string())
    } else {
        Some(display)
    }
}

unsafe fn populate_sequences_list(h_list: HWND) {
    if let Ok(names) = storage::list_sequences() {
        let bindings = hotkeys::current_sequence_bindings();
        for name in &names {
            let display = if let Some((vk, _)) = bindings.iter().find(|(_, n)| n == name) {
                format!("{} [{}]", name, vk_name(*vk))
            } else {
                name.clone()
            };
            let wname = wide(&display);
            SendMessageW(h_list, LB_ADDSTRING, 0, wname.as_ptr() as LPARAM);
        }
    }
}

/// Refresh the queue listbox.
unsafe fn refresh_queue_list(hwnd: HWND) {
    let h_list = GetDlgItem(hwnd, IDC_LIST_QUEUE as i32);
    if h_list.is_null() {
        return;
    }
    SendMessageW(h_list, LB_RESETCONTENT, 0, 0);
    let queue = lock_or_recover(&SEQUENCE_QUEUE);
    for (i, name) in queue.iter().enumerate() {
        let display = format!("{}. {}", i + 1, name);
        let wname = wide(&display);
        SendMessageW(h_list, LB_ADDSTRING, 0, wname.as_ptr() as LPARAM);
    }
}
