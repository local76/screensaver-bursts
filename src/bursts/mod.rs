//! Consolidated bursts screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

use crate::runner::core::LcgRng;
use crate::runner::toolkit::sys_info::get_system_info;

pub mod types;
pub mod physics;
pub mod drawing;
pub mod update;
pub mod draw;

use types::*;

pub struct Bursts {
    pub(crate) rng: LcgRng,
    pub(crate) rockets: Vec<Rocket>,
    pub(crate) particles: Vec<Particle>,
    pub(crate) stars: Vec<Star>,
    pub(crate) skyline: Vec<usize>, // Height of building at each column
    pub(crate) skyline_windows: Vec<bool>, // Whether window is lit at grid cell (r * cols + c)
    pub(crate) time_elapsed: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(crate) launch_rate_opt: u32,
    pub(crate) skyline_style_opt: u32,
    pub(crate) logo_text: String,

    // Live system dynamics
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) _host_bias: f32,
    pub(crate) on_battery: bool,
    pub(crate) frame_time_ema: f32,
    pub(crate) quality_scale: f32,
    pub(crate) target_frame_time: f32,
}

impl Default for Bursts {
    fn default() -> Self {
        Self::new()
    }
}

impl Bursts {
    pub fn new() -> Self {
        let launch_rate_opt: u32 = 1;
        let skyline_style_opt: u32 = 0;

        let sys = get_system_info();
        let logo_text = sys.logo_text.clone();
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;
        let on_battery = sys.power_status.contains("Battery");

        Self {
            rng: LcgRng::new(7777),
            rockets: Vec::new(),
            particles: Vec::new(),
            stars: Vec::new(),
            skyline: Vec::new(),
            skyline_windows: Vec::new(),
            time_elapsed: 0.0,
            last_cols: 0,
            last_rows: 0,
            launch_rate_opt,
            skyline_style_opt,
            logo_text,
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0),
            _host_bias: host_bias,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
        }
    }

    pub(crate) fn generate_skyline(&mut self, cols: usize, rows: usize) {
        self.skyline = vec![0; cols];
        self.skyline_windows = vec![false; cols * rows];

        if self.skyline_style_opt == 1 || rows < 4 || cols == 0 {
            return; // Empty sky or too small terminal
        }

        let mut c = 0;
        while c < cols {
            let building_w = self.rng.next_usize(6) + 3; // 3 to 8 cols wide
            let building_h = self.rng.next_usize(rows / 4) + 3; // building height

            for i in 0..building_w {
                if c + i < cols {
                    self.skyline[c + i] = building_h;
                    
                    // Windows in this building
                    for r in 0..building_h {
                        let gy = rows.saturating_sub(1).saturating_sub(r);
                        if self.rng.next_bool(0.12) {
                            self.skyline_windows[gy * cols + (c + i)] = true;
                        }
                    }
                }
            }
            c += building_w + self.rng.next_usize(2); // gap between buildings
        }
    }
}

#[cfg(test)]
#[path = "bursts_tests.rs"]
mod tests;
