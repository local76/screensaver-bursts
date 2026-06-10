//! Consolidated bursts screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

use library::core::{LcgRng, TerminalCell};
use std::time::Duration;
use library::core::screensaver::Screensaver;
use library::platform::native::sys_info::get_system_info;

pub mod types;
pub mod physics;

use types::*;

pub struct Bursts {
    rng: LcgRng,
    pub(crate) rockets: Vec<Rocket>,
    pub(crate) particles: Vec<Particle>,
    pub(crate) stars: Vec<Star>,
    pub(crate) skyline: Vec<usize>, // Height of building at each column
    pub(crate) skyline_windows: Vec<bool>, // Whether window is lit at grid cell (r * cols + c)
    pub(crate) time_elapsed: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    launch_rate_opt: u32,
    skyline_style_opt: u32,

    // Live system dynamics
    sys_refresh_timer: f32,
    mem_pressure: f32,
    cpu_load: f32,
    host_bias: f32,}

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
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;

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
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: 0.4,
            host_bias,
        }
    }

    fn generate_skyline(&mut self, cols: usize, rows: usize) {
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

impl Screensaver for Bursts {
    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        let delta = dt.as_secs_f32();
        self.time_elapsed += delta;

        // Live refresh: more launches under load, host_bias for variety
        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let sys = get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.cpu_load = (self.mem_pressure * 0.6 + 0.3).min(0.9);
            if self.host_bias > 0.65 { self.cpu_load = (self.cpu_load + 0.1).min(0.98); }
            self.sys_refresh_timer = 0.0;
        }

        // Initialize skyline if resized
        if cols != self.last_cols || rows != self.last_rows {
            self.generate_skyline(cols, rows);
            self.rockets.clear();
            self.particles.clear();
            
            // Create background stars
            let target_stars = (cols * rows / 20).clamp(10, 80);
            let mut stars = Vec::new();
            for i in 0..target_stars {
                stars.push(Star {
                    x: self.rng.next_f32(),
                    y: self.rng.next_f32(),
                    phase: self.rng.next_f32() * std::f32::consts::TAU,
                    ch: if i % 7 == 0 { '✦' } else if i % 3 == 0 { '•' } else { '.' },
                    excitation: 0.0,
                    excited_color: (255, 255, 255),
                });
            }
            self.stars = stars;

            self.last_cols = cols;
            self.last_rows = rows;
        }

        // 1. Launch new rockets randomly
        // Live: higher CPU/mem pressure = more frequent/larger fireworks show
        let load_mult = 1.0 + self.cpu_load * 0.7 + self.mem_pressure * 0.3;
        let (base_max, base_chance) = match self.launch_rate_opt {
            0 => (2, 0.015),
            2 => (7, 0.09),
            _ => (4, 0.04),
        };
        let max_rockets = (base_max as f32 * load_mult).max(1.0) as usize;
        let chance = base_chance * load_mult;
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
            let num_particles = self.rng.next_usize(20) + 20;
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

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        self.draw_impl(grid, cols, rows);
    }
}

impl Bursts {
    pub fn draw_impl(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        // Collect active explosions to light up the logo and buildings
        let mut active_explosions = Vec::new();
        for p in &self.particles {
            if p.color != (100, 100, 100) {
                let pct = p.life / p.max_life;
                if pct > 0.4 {
                    active_explosions.push(ActiveExplosion {
                        x: p.x,
                        y: p.y,
                        radius: 11.0 * pct,
                        color: p.color,
                        intensity: pct,
                    });
                }
            }
        }

        // 1. Draw background stars
        physics::draw_stars(
            &self.stars,
            &self.skyline,
            &active_explosions,
            self.time_elapsed,
            grid,
            cols,
            rows,
        );

        // 2. Draw centered logo
        let logo_text = get_system_info().logo_text;
        physics::draw_logo(
            &logo_text,
            &active_explosions,
            grid,
            cols,
            rows,
        );

        // 3. Draw rising rockets
        for rocket in &self.rockets {
            let cx = rocket.x as usize;
            let cy = rocket.y as usize;
            if cx < cols && cy < rows {
                grid[cy * cols + cx] = TerminalCell {
                    ch: '▲',
                    fg: (255, 255, 255),
                    bg: (0, 0, 0),
                    bold: true,
                };
            }
        }

        // 4. Draw explosion particles
        for p in &self.particles {
            let cx = p.x as usize;
            let cy = p.y as usize;
            if cx < cols && cy < rows {
                let pct = p.life / p.max_life;
                let color = (
                    (p.color.0 as f32 * pct) as u8,
                    (p.color.1 as f32 * pct) as u8,
                    (p.color.2 as f32 * pct) as u8,
                );
                
                // Only draw if skyward of the skyline profile (except smoke)
                let is_smoke = p.color == (100, 100, 100);
                let height_from_bottom = rows.saturating_sub(1).saturating_sub(cy);
                if height_from_bottom >= self.skyline[cx] || is_smoke {
                    // Only overwrite empty space or flares (except if smoke)
                    let current_ch = grid[cy * cols + cx].ch;
                    if is_smoke || current_ch == ' ' || current_ch == '─' || current_ch == '│' || current_ch == '/' || current_ch == '\\' {
                        grid[cy * cols + cx] = TerminalCell {
                            ch: p.ch,
                            fg: color,
                            bg: (0, 0, 0),
                            bold: pct > 0.5,
                        };
                    }
                }
            }
        }

        // 5. Draw city skyline (with building windows reacting to nearby explosions)
        physics::draw_skyline(
            &self.skyline,
            &self.skyline_windows,
            &active_explosions,
            grid,
            cols,
            rows,
        );

        // 6. Draw overlay cinematic lens flares and starbursts centered at active explosion origins
        let mut drawn_explosion_flares: Vec<(f32, f32)> = Vec::new();
        for p in &self.particles {
            if p.color != (100, 100, 100) {
                let pct = p.life / p.max_life;
                if pct > 0.85 {
                    let ex = p.x;
                    let ey = p.y;
                    
                    let mut too_close = false;
                    for &(dx, dy) in &drawn_explosion_flares {
                        let dist = ((ex - dx)*0.55).hypot(ey - dy);
                        if dist < 5.0 {
                            too_close = true;
                            break;
                        }
                    }
                    if too_close { continue; }
                    drawn_explosion_flares.push((ex, ey));

                    let sx = ex as usize;
                    let sy = ey as usize;
                    if sx < cols && sy < rows {
                        let flare_intensity = (pct - 0.85) / 0.15;
                        let (er, eg, eb) = p.color;

                        let center_idx = sy * cols + sx;
                        grid[center_idx] = TerminalCell {
                            ch: '✸',
                            fg: (255, 255, 255),
                            bg: grid[center_idx].bg,
                            bold: true,
                        };

                        // Draw horizontal streak
                        let h_len = 16;
                        for dx in 1..h_len {
                            let alpha = (160.0 * flare_intensity) as u8;
                            let fade = alpha.saturating_sub((dx * (150 / h_len)) as u8);
                            if fade > 15 {
                                if sx + dx < cols {
                                    let cell = &mut grid[sy * cols + (sx + dx)];
                                    cell.fg = physics::blend_explosion_flare_h_color(cell.fg, fade, (er, eg, eb));
                                    if cell.ch == ' ' {
                                        cell.ch = '─';
                                    }
                                }
                                if sx >= dx {
                                    let cell = &mut grid[sy * cols + (sx - dx)];
                                    cell.fg = physics::blend_explosion_flare_h_color(cell.fg, fade, (er, eg, eb));
                                    if cell.ch == ' ' {
                                        cell.ch = '─';
                                    }
                                }
                            }
                        }

                        // Draw vertical streak
                        let v_len = 6;
                        for dy in 1..v_len {
                            let alpha = (110.0 * flare_intensity) as u8;
                            let fade = alpha.saturating_sub((dy * (100 / v_len)) as u8);
                            if fade > 15 {
                                if sy + dy < rows {
                                    let cell = &mut grid[(sy + dy) * cols + sx];
                                    cell.fg = physics::blend_explosion_flare_v_color(cell.fg, fade, (er, eg, eb));
                                    if cell.ch == ' ' {
                                        cell.ch = '│';
                                    }
                                }
                                if sy >= dy {
                                    let cell = &mut grid[(sy - dy) * cols + sx];
                                    cell.fg = physics::blend_explosion_flare_v_color(cell.fg, fade, (er, eg, eb));
                                    if cell.ch == ' ' {
                                        cell.ch = '│';
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bursts_new() {
        let bursts = Bursts::new();
        assert_eq!(bursts.rockets.len(), 0);
        assert_eq!(bursts.particles.len(), 0);
        assert_eq!(bursts.stars.len(), 0);
    }

    #[test]
    fn test_bursts_update_and_draw() {
        let mut bursts = Bursts::new();
        bursts.update(Duration::from_millis(16), 80, 24);
        let mut grid = vec![TerminalCell::default(); 80 * 24];
        bursts.draw(&mut grid, 80, 24);
        // Completed without panic, at least some initialization should be done
        assert!(!bursts.stars.is_empty());
    }
}

