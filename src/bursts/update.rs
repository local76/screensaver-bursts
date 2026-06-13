//! Screensaver update implementation for Bursts.

use crate::runner::core::screensaver::Screensaver;
use crate::runner::toolkit::sys_info::get_system_info;
use std::time::Duration;
use super::Bursts;
use super::types::{Rocket, Particle, Star, FIREWORK_COLORS};
use super::physics;

impl Screensaver for Bursts {
    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        let dt_secs = dt.as_secs_f32();

        // Auto-detect high refresh rates during the startup phase
        if self.time_elapsed < 2.0 && dt_secs > 0.001 {
            if dt_secs < self.target_frame_time - 0.001 {
                self.target_frame_time = dt_secs;
            }
        }

        // Exponential moving average for frame time (alpha = 0.1)
        self.frame_time_ema = self.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;

        let speed_mult = if self.on_battery { 0.65 } else { 1.0 };
        let delta = dt_secs * speed_mult;
        self.time_elapsed += delta;

        // Adjust quality_scale based on frame time performance vs target
        if self.time_elapsed > 1.5 {
            if self.frame_time_ema > self.target_frame_time * 1.15 {
                self.quality_scale = (self.quality_scale - 0.15 * delta).max(0.20);
            } else if self.frame_time_ema < self.target_frame_time * 1.05 {
                self.quality_scale = (self.quality_scale + 0.04 * delta).min(1.0);
            }
        }

        // Live refresh: more launches under load, host_bias for variety
        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let sys = get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
            self.on_battery = sys.power_status.contains("Battery");
            self.sys_refresh_timer = 0.0;
        }

        // Initialize skyline if resized
        if cols != self.last_cols || rows != self.last_rows {
            self.generate_skyline(cols, rows);
            self.rockets.clear();
            self.particles.clear();
            self.stars.clear();
            self.last_cols = cols;
            self.last_rows = rows;
        }

        // Dynamically adjust star population to match target capacity
        let target_stars = (((cols * rows / 20).clamp(10, 80)) as f32 * self.quality_scale * (if self.on_battery { 0.55 } else { 1.0 })) as usize;
        if self.stars.len() > target_stars {
            self.stars.truncate(target_stars);
        } else if self.stars.len() < target_stars && target_stars > 0 {
            while self.stars.len() < target_stars {
                self.stars.push(Star {
                    x: self.rng.next_f32(),
                    y: self.rng.next_f32(),
                    phase: self.rng.next_f32() * std::f32::consts::TAU,
                    ch: if self.stars.len() % 7 == 0 { '✦' } else if self.stars.len() % 3 == 0 { '•' } else { '.' },
                    excitation: 0.0,
                    excited_color: (255, 255, 255),
                });
            }
        }

        // 1. Launch new rockets randomly
        // Live: higher CPU/mem pressure = more frequent/larger fireworks show
        let load_mult = 1.0 + self.cpu_load * 0.7 + self.mem_pressure * 0.3;
        let (base_max, base_chance) = match self.launch_rate_opt {
            0 => (2, 0.015),
            2 => (7, 0.09),
            _ => (4, 0.04),
        };
        let max_rockets = (base_max as f32 * load_mult * self.quality_scale * (if self.on_battery { 0.55 } else { 1.0 })).max(1.0) as usize;
        let chance = base_chance * load_mult * self.quality_scale * (if self.on_battery { 0.55 } else { 1.0 });
        if self.rockets.len() < max_rockets && self.rng.next_bool(chance) {
            let start_x = self.rng.next_range(5.0, cols as f32 - 5.0);
            let start_y = rows as f32 - 1.0;
            let target_y = self.rng.next_range(3.0, rows as f32 * 0.55);
            let color = FIREWORK_COLORS[self.rng.next_usize(FIREWORK_COLORS.len())];
            
            // Aim towards the middle
            let target_x = self.rng.next_range(cols as f32 * 0.25, cols as f32 * 0.75);
            let dx = target_x - start_x;
            let dy = target_y - start_y;
            let time_to_peak = self.rng.next_range(1.2, 1.8);
            let vx = dx / time_to_peak;
            let vy = dy / time_to_peak;

            self.rockets.push(Rocket {
                x: start_x,
                y: start_y,
                vx,
                vy,
                target_y,
                color,
            });
        }

        // 2. Update rockets
        let mut exploded_rockets = Vec::new();
        for (i, rocket) in self.rockets.iter_mut().enumerate() {
            rocket.x += rocket.vx * delta;
            rocket.y += rocket.vy * delta;

            // Spawn smoke/trail particles
            if self.rng.next_bool(0.4) {
                self.particles.push(Particle {
                    x: rocket.x,
                    y: rocket.y,
                    vx: self.rng.next_range(-0.5, 0.5),
                    vy: self.rng.next_range(0.2, 1.0),
                    color: (100, 100, 100),
                    ch: '.',
                    life: 0.6,
                    max_life: 0.6,
                });
            }

            if rocket.y <= rocket.target_y {
                exploded_rockets.push(i);
            }
        }

        // Process explosions
        for idx in exploded_rockets.into_iter().rev() {
            let rocket = self.rockets.remove(idx);
            // Spawn explosion particles
            let num_particles = ((self.rng.next_usize(20) + 20) as f32 * self.quality_scale * (if self.on_battery { 0.55 } else { 1.0 })) as usize;
            for _ in 0..num_particles {
                let angle = self.rng.next_range(0.0, std::f32::consts::TAU);
                let speed = self.rng.next_range(4.0, 16.0);
                
                let vx = angle.cos() * speed / 0.55;
                let vy = angle.sin() * speed;

                let ch = match self.rng.next_usize(4) {
                    0 => '*',
                    1 => '+',
                    2 => '•',
                    _ => '.',
                };
                let max_life = self.rng.next_range(0.8, 1.5);

                self.particles.push(Particle {
                    x: rocket.x,
                    y: rocket.y,
                    vx,
                    vy,
                    color: rocket.color,
                    ch,
                    life: max_life,
                    max_life,
                });
            }
        }

        // Decay star excitations
        for star in &mut self.stars {
            if star.excitation > 0.0 {
                star.excitation -= delta * 2.0;
                if star.excitation < 0.0 {
                    star.excitation = 0.0;
                }
            }
        }

        // 3. Update explosion particles and check star excitation
        physics::update_particles_and_excite_stars(
            &mut self.particles,
            &mut self.stars,
            delta,
            cols,
            rows,
        );

        self.particles.retain(|p| p.life > 0.0);
    }

    fn draw(&self, grid: &mut [crate::runner::core::TerminalCell], cols: usize, rows: usize) {
        self.draw_impl(grid, cols, rows);
    }
}
