//! Cross-platform terminal/screen helpers used by the screensaver runner.
//! Each function has a Windows and a non-Windows variant selected by `cfg`.

// ---------------------------------------------------------------------------
// Monitor refresh rate
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub fn get_monitor_refresh_rate() -> u32 {
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC, GetDeviceCaps};
        let hdc = GetDC(std::ptr::null_mut());
        if !hdc.is_null() {
            let rate = GetDeviceCaps(hdc, 116); // VREFRESH
            ReleaseDC(std::ptr::null_mut(), hdc);
            if rate > 0 {
                return rate as u32;
            }
        }
    }
    60
}

#[cfg(not(target_os = "windows"))]
pub fn get_monitor_refresh_rate() -> u32 {
    120
}

// ---------------------------------------------------------------------------
// Terminal size
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub fn get_terminal_size() -> (usize, usize) {
    unsafe {
        use windows_sys::Win32::System::Console::{
            GetStdHandle, GetConsoleScreenBufferInfo, CONSOLE_SCREEN_BUFFER_INFO,
            STD_OUTPUT_HANDLE,
        };
        let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        if GetConsoleScreenBufferInfo(stdout_handle, &mut info) != 0 {
            let cols = (info.srWindow.Right - info.srWindow.Left + 1) as usize;
            let rows = (info.srWindow.Bottom - info.srWindow.Top + 1) as usize;
            (cols, rows)
        } else {
            (80, 24)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_terminal_size() -> (usize, usize) {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 {
            (ws.ws_col as usize, ws.ws_row as usize)
        } else {
            (80, 24)
        }
    }
}

// ---------------------------------------------------------------------------
// Mouse activity
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub fn check_mouse_activity(initial_pos: &mut Option<(i32, i32)>) -> bool {
    unsafe {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut pt: POINT = std::mem::zeroed();
        if GetCursorPos(&mut pt) != 0 {
            if let Some((init_x, init_y)) = *initial_pos {
                let dx = (pt.x - init_x).abs();
                let dy = (pt.y - init_y).abs();
                if dx > 10 || dy > 10 {
                    return true;
                }
            } else {
                *initial_pos = Some((pt.x, pt.y));
            }
        }
        if GetAsyncKeyState(0x01) != 0 || GetAsyncKeyState(0x02) != 0 || GetAsyncKeyState(0x04) != 0 {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub fn check_mouse_activity(_initial_pos: &mut Option<(i32, i32)>) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Keypress detection
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub fn check_keypress() -> bool {
    unsafe {
        use windows_sys::Win32::System::Console::{
            GetStdHandle, PeekConsoleInputW, ReadConsoleInputW,
            GetNumberOfConsoleInputEvents, INPUT_RECORD, KEY_EVENT, STD_INPUT_HANDLE,
        };
        let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
        let mut num_events = 0;
        if GetNumberOfConsoleInputEvents(stdin_handle, &mut num_events) == 0 {
            return false;
        }
        if num_events == 0 {
            return false;
        }
        let mut buffer: [INPUT_RECORD; 128] = std::mem::zeroed();
        let mut read = 0;
        if PeekConsoleInputW(stdin_handle, buffer.as_mut_ptr(), num_events.min(128), &mut read) != 0 {
            for i in 0..read as usize {
                let rec = &buffer[i];
                if rec.EventType == KEY_EVENT as u16 {
                    let key_event = &rec.Event.KeyEvent;
                    if key_event.bKeyDown != 0 {
                        let mut temp: [INPUT_RECORD; 1] = std::mem::zeroed();
                        let mut num_read = 0;
                        let _ = ReadConsoleInputW(stdin_handle, temp.as_mut_ptr(), 1, &mut num_read);
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub fn check_keypress() -> bool {
    unsafe {
        let mut fd_set: libc::fd_set = std::mem::zeroed();
        libc::FD_SET(libc::STDIN_FILENO, &mut fd_set);
        let mut timeout = libc::timeval { tv_sec: 0, tv_usec: 0 };
        libc::select(
            libc::STDIN_FILENO + 1,
            &mut fd_set,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut timeout,
        ) > 0
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
pub fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
