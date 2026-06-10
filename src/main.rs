#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod bursts;

fn main() {
    let effect = bursts::Bursts::new();
    library::screensaver_runner::run_main(effect, "bursts");
}
