//! RawTerminalGuard: enters raw terminal mode on startup, restores on drop.

#[cfg(target_os = "windows")]
mod windows_impl {
    use windows_sys::Win32::System::Console::{
        GetStdHandle, GetConsoleMode, SetConsoleMode,
        ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, AllocConsole, GetConsoleWindow,
        STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, GetWindowRect, GetSystemMetrics,
        SetWindowLongW, SetWindowPos, ShowWindow,
    };

    pub struct RawTerminalGuard {
        pub stdin_handle: windows_sys::Win32::Foundation::HANDLE,
        pub stdout_handle: windows_sys::Win32::Foundation::HANDLE,
        pub original_in_mode: u32,
        pub original_out_mode: u32,
        pub hwnd: windows_sys::Win32::Foundation::HWND,
        pub original_style: i32,
        pub original_rect: [i32; 4],
    }

    impl RawTerminalGuard {
        pub fn enable() -> Option<Self> {
            unsafe {
                AllocConsole();
                let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
                let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
                if stdin_handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
                    || stdout_handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
                {
                    return None;
                }
                let mut original_in_mode = 0;
                if GetConsoleMode(stdin_handle, &mut original_in_mode) == 0 {
                    return None;
                }
                let mut original_out_mode = 0;
                let _ = GetConsoleMode(stdout_handle, &mut original_out_mode);

                let raw_in_mode = original_in_mode
                    & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT);
                if SetConsoleMode(stdin_handle, raw_in_mode) == 0 {
                    return None;
                }
                let raw_out_mode = original_out_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                let _ = SetConsoleMode(stdout_handle, raw_out_mode);

                let _ = timeBeginPeriod(1);

                let hwnd = GetConsoleWindow();
                let mut original_style = 0;
                let mut original_rect = [0; 4];
                if !hwnd.is_null() {
                    original_style = GetWindowLongW(hwnd, -16); // GWL_STYLE
                    let mut rect: RECT = std::mem::zeroed();
                    if GetWindowRect(hwnd, &mut rect) != 0 {
                        original_rect = [
                            rect.left, rect.top,
                            rect.right - rect.left,
                            rect.bottom - rect.top,
                        ];
                    }
                    // Strip WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX |
                // WS_MAXIMIZEBOX | WS_SYSMENU
                let new_style = original_style
                    & !(0x00C0_0000 | 0x0004_0000 | 0x0002_0000 | 0x0001_0000 | 0x0008_0000);
                SetWindowLongW(hwnd, -16, new_style);
                    let width = GetSystemMetrics(0);  // SM_CXSCREEN
                    let height = GetSystemMetrics(1); // SM_CYSCREEN
                    // SWP_FRAMECHANGED | SWP_SHOWWINDOW
                    SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, width, height, 0x0020 | 0x0040);
                    ShowWindow(hwnd, 3); // SW_MAXIMIZE
                }
                Some(Self {
                    stdin_handle,
                    stdout_handle,
                    original_in_mode,
                    original_out_mode,
                    hwnd,
                    original_style,
                    original_rect,
                })
            }
        }
    }

    impl Drop for RawTerminalGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = SetConsoleMode(self.stdin_handle, self.original_in_mode);
                let _ = SetConsoleMode(self.stdout_handle, self.original_out_mode);
                if !self.hwnd.is_null() {
                    SetWindowLongW(self.hwnd, -16, self.original_style);
                    SetWindowPos(
                        self.hwnd,
                        std::ptr::null_mut(),
                        self.original_rect[0],
                        self.original_rect[1],
                        self.original_rect[2],
                        self.original_rect[3],
                        0x0020 | 0x0040,
                    );
                }
                let _ = timeEndPeriod(1);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod linux_impl {
    pub struct RawTerminalGuard {
        pub original_termios: libc::termios,
        pub window_id: Option<u64>,
    }

    impl RawTerminalGuard {
        pub fn enable() -> Option<Self> {
            unsafe {
                let mut termios: libc::termios = std::mem::zeroed();
                if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) != 0 {
                    return None;
                }
                let original_termios = termios;
                libc::cfmakeraw(&mut termios);
                if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios) != 0 {
                    return None;
                }
                use std::io::Write;
                print!("\x1b[?1003h");
                let _ = std::io::stdout().flush();

                let window_id = if let Ok(win_id_str) = std::env::var("WINDOWID") {
                    if let Ok(win_id) = win_id_str.parse::<u64>() {
                        x11::hide_cursor_for_window(win_id);
                        Some(win_id)
                    } else {
                        None
                    }
                } else {
                    None
                };
                Some(Self { original_termios, window_id })
            }
        }
    }

    impl Drop for RawTerminalGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(win_id) = self.window_id {
                    x11::show_cursor_for_window(win_id);
                }
                use std::io::Write;
                print!("\x1b[?1003l");
                let _ = std::io::stdout().flush();
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original_termios);
            }
        }
    }

    mod x11 {
        unsafe extern "C" fn silent_error_handler(
            _display: *mut libc::c_void,
            _error_event: *mut libc::c_void,
        ) -> libc::c_int {
            0
        }

        pub unsafe fn hide_cursor_for_window(window_id: u64) {
            toggle_cursor(window_id, true)
        }
        pub unsafe fn show_cursor_for_window(window_id: u64) {
            toggle_cursor(window_id, false)
        }

        unsafe fn toggle_cursor(window_id: u64, hide: bool) {
            unsafe {
                let libx11 = libc::dlopen(
                    b"libX11.so.6\0".as_ptr() as *const libc::c_char,
                    libc::RTLD_LAZY,
                );
                if libx11.is_null() {
                    return;
                }
                let libxfixes = libc::dlopen(
                    b"libXfixes.so.3\0".as_ptr() as *const libc::c_char,
                    libc::RTLD_LAZY,
                );
                if libxfixes.is_null() {
                    libc::dlclose(libx11);
                    return;
                }

                let x_open: Option<unsafe extern "C" fn(*const libc::c_char) -> *mut libc::c_void> =
                    std::mem::transmute(libc::dlsym(libx11, b"XOpenDisplay\0".as_ptr() as *const _));
                let x_close: Option<unsafe extern "C" fn(*mut libc::c_void) -> libc::c_int> =
                    std::mem::transmute(libc::dlsym(libx11, b"XCloseDisplay\0".as_ptr() as *const _));
                let x_flush: Option<unsafe extern "C" fn(*mut libc::c_void) -> libc::c_int> =
                    std::mem::transmute(libc::dlsym(libx11, b"XFlush\0".as_ptr() as *const _));
                let x_sync: Option<unsafe extern "C" fn(*mut libc::c_void, libc::c_int) -> libc::c_int> =
                    std::mem::transmute(libc::dlsym(libx11, b"XSync\0".as_ptr() as *const _));
                let x_set_handler: Option<unsafe extern "C" fn(Option<unsafe extern "C" fn(*mut libc::c_void, *mut libc::c_void) -> libc::c_int>) -> *mut libc::c_void> =
                    std::mem::transmute(libc::dlsym(libx11, b"XSetErrorHandler\0".as_ptr() as *const _));
                let x_toggle: Option<unsafe extern "C" fn(*mut libc::c_void, u64)> = std::mem::transmute(
                    libc::dlsym(
                        libxfixes,
                        if hide { b"XFixesHideCursor\0".as_ptr() as *const _ }
                        else { b"XFixesShowCursor\0".as_ptr() as *const _ },
                    ),
                );

                if let (Some(open), Some(close), Some(toggle)) = (x_open, x_close, x_toggle) {
                    let display = open(std::ptr::null());
                    if !display.is_null() {
                        if let Some(set_handler) = x_set_handler {
                            set_handler(Some(silent_error_handler));
                        }
                        toggle(display, window_id);
                        if let Some(sync) = x_sync {
                            sync(display, 0);
                        } else if let Some(flush) = x_flush {
                            flush(display);
                        }
                        if let Some(set_handler) = x_set_handler {
                            set_handler(None);
                        }
                        close(display);
                    }
                }
                libc::dlclose(libxfixes);
                libc::dlclose(libx11);
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::RawTerminalGuard;
#[cfg(not(target_os = "windows"))]
pub use linux_impl::RawTerminalGuard;

