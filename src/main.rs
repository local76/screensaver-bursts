#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod runner;
mod bursts;

fn main() {
    let effect = bursts::Bursts::new();
    runner::run_main(effect, "bursts");
}