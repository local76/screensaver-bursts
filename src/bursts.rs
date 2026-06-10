//! Consolidated bursts screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).


use library::core::{LcgRng, TerminalCell};
use std::time::Duration;
use library::core::screensaver::Screensaver;

use library::platform::native::sys_info::get_system_info;

use library::toolkit::rgb_controller::{RgbController, is_openrgb_enabled};

use library::toolkit::rgb_protocol::RgbColor;
use library::core::logo_block::render_logo_block;

pub struct Rocket {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub target_y: f32,
    pub color: (u8, u8, u8),
}

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: (u8, u8, u8),
    pub ch: char,
    pub life: f32,
    pub max_life: f32,
}

pub struct ActiveExplosion {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: (u8, u8, u8),
    pub intensity: f32,
}

pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub ch: char,
    pub excitation: f32,
    pub excited_color: (u8, u8, u8),
}

pub const FIREWORK_COLORS: &[(u8, u8, u8)] = &[
    (255, 50, 50),   // Red
    (50, 255, 50),   // Green
    (50, 100, 255),  // Blue
    (255, 200, 0),   // Gold/Yellow
    (255, 50, 255),  // Magenta/Pink
    (0, 255, 255),   // Cyan
    (255, 128, 0),   // Orange
    (200, 200, 255), // Silver
];


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
    host_bias: f32,
    rgb: Option<RgbController>,
}

impl Default for Bursts {
    fn default() -> Self {
        Self::new()
    }
}

impl Bursts {
    pub fn new() -> Self {
        // Pre-4.1 HKEY_CURRENT_USER registry reads (LaunchRate, SkylineStyle)
        // collapsed to defaults for the inline migration. Re-added in 4.2.
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
            rgb: if is_openrgb_enabled() { Some(RgbController::new()) } else { None },
        }
    }

    fn generate_skyline(&mut self, cols: usize, rows: usize) {
        self.skyline = vec![0; cols];
        self.skyline_windows = vec![false; cols * rows];

        if self.skyline_style_opt == 1 {
            return; // Empty sky
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
            if let Some(ref r) = self.rgb {
                let color = RgbColor::new(rocket.color.0, rocket.color.1, rocket.color.2);
                r.flash(color, std::time::Duration::from_millis(300));
            }
            
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
        let cols_f = cols as f32;
        let rows_f = rows as f32;
        for p in &mut self.particles {
            p.x += p.vx * delta;
            p.y += p.vy * delta;
            
            p.vy += 4.5 * delta; // Gravity
            p.vx *= 1.0 - 0.5 * delta; // Drag
            p.vy *= 1.0 - 0.5 * delta;

            p.life -= delta;

            // Excite background stars (ignore smoke particles)
            if p.color != (100, 100, 100) && p.life > 0.0 {
                for star in &mut self.stars {
                    let sx = star.x * cols_f;
                    let sy = star.y * rows_f;
                    let dx = p.x - sx;
                    let dy = (p.y - sy) * 2.0;
                    let dist_sq = dx*dx + dy*dy;
                    if dist_sq < 9.0 {
                        let dist = dist_sq.sqrt();
                        let force = (1.0 - dist / 3.0) * 1.5;
                        if force > star.excitation {
                            star.excitation = force;
                            star.excited_color = p.color;
                        }
                    }
                }
            }
        }
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

        // Find top candidates for lens flares (only highly excited stars, max 4)
        let mut flare_candidates: Vec<(usize, f32)> = self.stars.iter()
            .enumerate()
            .filter(|(_, star)| star.excitation > 0.8)
            .map(|(idx, star)| (idx, star.excitation))
            .collect();
        flare_candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
        let allowed_flares: Vec<usize> = flare_candidates.iter()
            .take(4)
            .map(|&(idx, _)| idx)
            .collect();

        // Draw background stars (illuminated by active explosions and excited by sparks)
        for (i, star) in self.stars.iter().enumerate() {
            let sx = (star.x * cols as f32) as usize;
            let sy = (star.y * rows as f32) as usize;
            if sx < cols && sy < rows {
                // Only draw if skyward of the skyline profile
                let height_from_bottom = rows.saturating_sub(1).saturating_sub(sy);
                if height_from_bottom >= self.skyline[sx] {
                    // Check light from active explosions
                    let mut best_intensity = 0.0f32;
                    let mut lit_color = (0, 0, 0);

                    for exp in &active_explosions {
                        let dx = (sx as f32 - exp.x) * 0.55;
                        let dy = sy as f32 - exp.y;
                        let dist = (dx*dx + dy*dy).sqrt();
                        if dist < exp.radius {
                            let intensity = (1.0 - dist / exp.radius) * exp.intensity;
                            if intensity > best_intensity {
                                best_intensity = intensity;
                                lit_color = (
                                    (exp.color.0 as f32 * intensity) as u8,
                                    (exp.color.1 as f32 * intensity) as u8,
                                    (exp.color.2 as f32 * intensity) as u8,
                                );
                            }
                        }
                    }

                    // Base twinkle brightness
                    let sparkle_base = ((self.time_elapsed * 2.0 + star.phase).sin() + 1.0) * 0.5;
                    let sparkle = (sparkle_base + star.excitation).min(2.0);
                    let base_brightness = (sparkle_base * 120.0 + 40.0) as u8;

                    // Blend base color (dim white) with explosion/excited color
                    let mut r = base_brightness.saturating_add((lit_color.0 as f32 * 0.8) as u8);
                    let mut g = base_brightness.saturating_add((lit_color.1 as f32 * 0.8) as u8);
                    let mut b = (base_brightness.saturating_add(25)).saturating_add((lit_color.2 as f32 * 0.8) as u8);

                    if star.excitation > 0.05 {
                        let blend = (star.excitation * 0.7).min(1.0);
                        r = (r as f32 * (1.0 - blend) + star.excited_color.0 as f32 * blend).min(255.0) as u8;
                        g = (g as f32 * (1.0 - blend) + star.excited_color.1 as f32 * blend).min(255.0) as u8;
                        b = (b as f32 * (1.0 - blend) + star.excited_color.2 as f32 * blend).min(255.0) as u8;
                    }

                    let final_brightness = sparkle * 0.4 + best_intensity * 0.6;

                    let ch = if final_brightness > 0.8 {
                        '✹'
                    } else if final_brightness > 0.5 {
                        '✦'
                    } else {
                        star.ch
                    };

                    grid[sy * cols + sx] = TerminalCell {
                        ch,
                        fg: (r, g, b),
                        bg: (0, 0, 0),
                        bold: final_brightness > 0.6 || star.excitation > 0.3,
                    };

                    // Draw lens flares and starbursts on highly illuminated/excited stars
                    let is_excited = allowed_flares.contains(&i);
                    if is_excited {
                        let flare_intensity = ((star.excitation - 0.8) / 0.7 + 0.5).min(1.5);
                        let flare_color = star.excited_color;

                        // Draw horizontal flare (cinematic anamorphic streak, longer)
                        let h_len = 12;
                        for dx in 1..h_len {
                            let alpha = (120.0 * flare_intensity).max(30.0) as u8;
                            let fade = alpha.saturating_sub((dx * (110 / h_len)) as u8);
                            if fade > 10 {
                                if sx + dx < cols {
                                    let cell = &mut grid[sy * cols + (sx + dx)];
                                    let h_test = rows.saturating_sub(1).saturating_sub(sy);
                                    if h_test >= self.skyline[sx + dx] && (cell.ch == ' ' || cell.ch == '─') {
                                        cell.ch = '─';
                                        let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                        let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                        let fg_b = (fade.saturating_add(45)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                        cell.fg = (fg_r, fg_g, fg_b);
                                    }
                                }
                                if sx >= dx {
                                    let cell = &mut grid[sy * cols + (sx - dx)];
                                    let h_test = rows.saturating_sub(1).saturating_sub(sy);
                                    if h_test >= self.skyline[sx - dx] && (cell.ch == ' ' || cell.ch == '─') {
                                        cell.ch = '─';
                                        let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                        let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                        let fg_b = (fade.saturating_add(45)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                        cell.fg = (fg_r, fg_g, fg_b);
                                    }
                                }
                            }
                        }

                        // Draw vertical flare
                        let v_len = 5;
                        for dy in 1..v_len {
                            let alpha = (90.0 * flare_intensity).max(20.0) as u8;
                            let fade = alpha.saturating_sub((dy * (80 / v_len)) as u8);
                            if fade > 10 {
                                if sy + dy < rows {
                                    let cell = &mut grid[(sy + dy) * cols + sx];
                                    let h_test = rows.saturating_sub(1).saturating_sub(sy + dy);
                                    if h_test >= self.skyline[sx] && (cell.ch == ' ' || cell.ch == '│') {
                                        cell.ch = '│';
                                        let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                        let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                        let fg_b = (fade.saturating_add(30)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                        cell.fg = (fg_r, fg_g, fg_b);
                                    }
                                }
                                if sy >= dy {
                                    let cell = &mut grid[(sy - dy) * cols + sx];
                                    let h_test = rows.saturating_sub(1).saturating_sub(sy - dy);
                                    if h_test >= self.skyline[sx] && (cell.ch == ' ' || cell.ch == '│') {
                                        cell.ch = '│';
                                        let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                        let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                        let fg_b = (fade.saturating_add(30)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                        cell.fg = (fg_r, fg_g, fg_b);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // library 4.1: render the centered system logo from the live OS info
        // (replaces pre-4.1 `trance_core::logo_lines()` + `logo_dimensions()`).
        let logo_text = get_system_info().logo_text;
        let lines = render_logo_block(&logo_text, None);
        let logo_h = lines.len();
        let logo_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let logo_x = cols.saturating_sub(logo_w) / 2;
        let logo_y = rows.saturating_sub(logo_h) / 2;

        for (r_offset, line) in lines.iter().enumerate().take(logo_h) {
            let gy = logo_y + r_offset;
            if gy >= rows { continue; }
            for (c_offset, ch) in line.chars().enumerate() {
                let gx = logo_x + c_offset;
                if gx >= cols { continue; }
                if ch != ' ' {
                    // Check light from active explosions
                    let mut best_intensity = 0.0f32;
                    let mut lit_color = (35, 15, 50); // dim default silhouette color

                    for exp in &active_explosions {
                        let dx = (gx as f32 - exp.x) * 0.55;
                        let dy = gy as f32 - exp.y;
                        let dist = (dx*dx + dy*dy).sqrt();
                        if dist < exp.radius {
                            let intensity = (1.0 - dist / exp.radius) * exp.intensity;
                            if intensity > best_intensity {
                                best_intensity = intensity;
                                lit_color = (
                                    (exp.color.0 as f32 * intensity + 35.0 * (1.0 - intensity)) as u8,
                                    (exp.color.1 as f32 * intensity + 15.0 * (1.0 - intensity)) as u8,
                                    (exp.color.2 as f32 * intensity + 50.0 * (1.0 - intensity)) as u8,
                                );
                            }
                        }
                    }

                    grid[gy * cols + gx] = TerminalCell {
                        ch,
                        fg: lit_color,
                        bg: (0, 0, 0),
                        bold: best_intensity > 0.1,
                    };
                }
            }
        }

        // 2. Draw rising rockets
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

        // 3. Draw explosion particles
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

        // 4. Draw city skyline (with building windows reacting to nearby explosions)
        for c in 0..cols {
            let building_h = self.skyline[c];
            for r in 0..building_h {
                let gy = rows.saturating_sub(1).saturating_sub(r);
                let idx = gy * cols + c;

                let mut ch = if self.skyline_windows[idx] {
                    '■'
                } else {
                    ' '
                };
                let mut fg = if self.skyline_windows[idx] {
                    (255, 220, 100)
                } else {
                    (0, 0, 0)
                };

                // Let windows dynamically reflect the color of nearby explosions
                let mut best_intensity = 0.0f32;
                let mut glow_color = (0, 0, 0);

                for exp in &active_explosions {
                    let dx = (c as f32 - exp.x) * 0.55;
                    let dy = gy as f32 - exp.y;
                    let dist = (dx*dx + dy*dy).sqrt();
                    let glow_radius = exp.radius * 2.2;
                    if dist < glow_radius {
                        let intensity = (1.0 - dist / glow_radius) * exp.intensity * 0.95;
                        if intensity > best_intensity {
                            best_intensity = intensity;
                            glow_color = exp.color;
                        }
                    }
                }

                if best_intensity > 0.05 {
                    if self.skyline_windows[idx] {
                        // Lit windows shift color based on the explosion's flash
                        fg = (
                            (fg.0 as f32 * (1.0 - best_intensity) + glow_color.0 as f32 * best_intensity).min(255.0) as u8,
                            (fg.1 as f32 * (1.0 - best_intensity) + glow_color.1 as f32 * best_intensity).min(255.0) as u8,
                            (fg.2 as f32 * (1.0 - best_intensity) + glow_color.2 as f32 * best_intensity).min(255.0) as u8,
                        );
                    } else {
                        // Unlit windows illuminate temporarily as the explosion flashes over them!
                        ch = '■';
                        fg = (
                            (glow_color.0 as f32 * best_intensity * 0.38) as u8,
                            (glow_color.1 as f32 * best_intensity * 0.38) as u8,
                            (glow_color.2 as f32 * best_intensity * 0.38) as u8,
                        );
                    }
                }

                grid[idx] = TerminalCell {
                    ch,
                    fg,
                    bg: (15, 15, 22), // deep dark skyline gray-blue
                    bold: best_intensity > 0.2,
                };
            }
        }

        // 5. Draw overlay cinematic lens flares and starbursts centered at active explosion origins
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
                                let blend_color = |cell: &TerminalCell| -> (u8, u8, u8) {
                                    (
                                        cell.fg.0.saturating_add((er as f32 * (fade as f32 / 255.0)) as u8),
                                        cell.fg.1.saturating_add((eg as f32 * (fade as f32 / 255.0)) as u8),
                                        cell.fg.2.saturating_add((eb as f32 * (fade as f32 / 255.0) + 40.0 * (fade as f32 / 255.0)) as u8),
                                    )
                                };
                                if sx + dx < cols {
                                    let cell = &mut grid[sy * cols + (sx + dx)];
                                    cell.fg = blend_color(cell);
                                    if cell.ch == ' ' {
                                        cell.ch = '─';
                                    }
                                }
                                if sx >= dx {
                                    let cell = &mut grid[sy * cols + (sx - dx)];
                                    cell.fg = blend_color(cell);
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
                                let blend_color = |cell: &TerminalCell| -> (u8, u8, u8) {
                                    (
                                        cell.fg.0.saturating_add((er as f32 * (fade as f32 / 255.0)) as u8),
                                        cell.fg.1.saturating_add((eg as f32 * (fade as f32 / 255.0)) as u8),
                                        cell.fg.2.saturating_add((eb as f32 * (fade as f32 / 255.0) + 20.0 * (fade as f32 / 255.0)) as u8),
                                    )
                                };
                                if sy + dy < rows {
                                    let cell = &mut grid[(sy + dy) * cols + sx];
                                    cell.fg = blend_color(cell);
                                    if cell.ch == ' ' {
                                        cell.ch = '│';
                                    }
                                }
                                if sy >= dy {
                                    let cell = &mut grid[(sy - dy) * cols + sx];
                                    cell.fg = blend_color(cell);
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
