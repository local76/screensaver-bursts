//! Cross-platform screensaver runtime host.
//! Vendored from `runner::screensaver_runner`.

use std::time::Duration;
use crate::runner::core::TerminalCell;
use crate::runner::core::screensaver::Screensaver;

#[path = "args.rs"]
mod args;
#[path = "platform_helpers.rs"]
mod platform_helpers;
#[path = "renderer.rs"]
mod renderer;
#[path = "terminal_guard.rs"]
mod terminal_guard;

pub use args::{parse_args, print_usage, Mode};
/// Run the screensaver with the given effect.
pub fn run_main<S: Screensaver + 'static>(saver: S, name: &str) {
    let mode = parse_args();
    match mode {
        Mode::Run => {
            let code = run_fullscreen(saver);
            std::process::exit(code as i32);
        }
        Mode::Configure => {
            eprintln!("({name}) configuration dialog: not yet implemented.");
            std::process::exit(0);
        }
        Mode::Preview => {
            #[cfg(target_os = "windows")]
            {
                let code = run_preview_stub(saver);
                std::process::exit(code as i32);
            }
            #[cfg(not(target_os = "windows"))]
            {
                let code = run_fullscreen(saver);
                std::process::exit(code as i32);
            }
        }
        Mode::ShowUsage => {
            print_usage(name);
            std::process::exit(0);
        }
    }
}

#[cfg(target_os = "windows")]
fn run_preview_stub<S: Screensaver + 'static>(_saver: S) -> isize {
    eprintln!("Windows preview mode is not supported in console mode.");
    0
}

// ---------------------------------------------------------------------------
// Common Fullscreen Animation Loop
// ---------------------------------------------------------------------------

fn run_fullscreen<S: Screensaver + 'static>(mut saver: S) -> isize {
    #[cfg(not(target_os = "windows"))]
    {
        // If running under xscreensaver, embed the terminal window inside the target X11 window.
        if let Ok(win_id) = std::env::var("XSCREENSAVER_WINDOW") {
            if std::env::var("TRANCE_EMBEDDED").is_err() {
                if platform_helpers::command_exists("xterm") {
                    let self_exe = std::env::current_exe().unwrap_or_default();
                    let mut cmd = std::process::Command::new("xterm");
                    cmd.args(["-into", &win_id, "-geometry", "120x40", "-bg", "black", "-fg", "white", "-ms", "black", "-xrm", "XTerm*pointerColorBackground: black", "-xrm", "XTerm*pointerMode: 2", "-e"])
                        .arg(&self_exe)
                        .arg("/s")
                        .env("TRANCE_EMBEDDED", "1")
                        .env("XCURSOR_PATH", "/nonexistent")
                        .env("XCURSOR_THEME", "none");
                    if let Ok(mut child) = cmd.spawn() {
                        let _ = child.wait();
                        return 0;
                    }
                }
            }
        }
    }

    let _raw_mode = match terminal_guard::RawTerminalGuard::enable() {
        Some(g) => g,
        None => {
            eprintln!("screensaver: could not enter raw mode; aborting.");
            return 1;
        }
    };
    let (mut cols, mut rows) = platform_helpers::get_terminal_size();
    saver.init(cols, rows);

    let mut r = renderer::Renderer::new(cols, rows);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = std::time::Instant::now();

    // Query actual monitor refresh rate
    let target_fps = platform_helpers::get_monitor_refresh_rate();
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);

    let mut initial_mouse_pos = None;
    let start_time = std::time::Instant::now();

    loop {
        if platform_helpers::check_keypress() {
            break;
        }

        // Prevent instant exit on startup due to initial mouse shake or clicks
        if start_time.elapsed() > Duration::from_millis(500) {
            if platform_helpers::check_mouse_activity(&mut initial_mouse_pos) {
                break;
            }
        }

        // Handle terminal resize dynamically
        let (new_cols, new_rows) = platform_helpers::get_terminal_size();
        if new_cols != cols || new_rows != rows {
            cols = new_cols;
            rows = new_rows;
            grid = vec![TerminalCell::default(); cols * rows];
            saver.init(cols, rows);
            r = renderer::Renderer::new(cols, rows);
        }

        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame);
        last_frame = now;

        saver.update(dt, cols, rows);
        saver.draw(&mut grid, cols, rows);
        r.render_grid(&grid, cols, rows, saver.has_scanlines());

        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
    0
}
