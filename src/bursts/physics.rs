//! Helper functions and core calculations for the bursts screensaver.

use library::core::TerminalCell;
use library::core::logo_block::render_logo_block;
use super::types::{ActiveExplosion, Particle, Star};

pub fn calculate_star_illumination(
    sx: f32,
    sy: f32,
    active_explosions: &[ActiveExplosion],
) -> (f32, (u8, u8, u8)) {
    let mut best_intensity = 0.0f32;
    let mut lit_color = (0, 0, 0);

    for exp in active_explosions {
        let dx = (sx - exp.x) * 0.55;
        let dy = sy - exp.y;
        let dist = (dx * dx + dy * dy).sqrt();
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
    (best_intensity, lit_color)
}

pub fn calculate_logo_illumination(
    gx: f32,
    gy: f32,
    active_explosions: &[ActiveExplosion],
) -> (f32, (u8, u8, u8)) {
    let mut best_intensity = 0.0f32;
    let mut lit_color = (35, 15, 50); // dim default silhouette color

    for exp in active_explosions {
        let dx = (gx - exp.x) * 0.55;
        let dy = gy - exp.y;
        let dist = (dx * dx + dy * dy).sqrt();
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
    (best_intensity, lit_color)
}

pub fn calculate_window_illumination(
    cx: f32,
    cy: f32,
    active_explosions: &[ActiveExplosion],
) -> (f32, (u8, u8, u8)) {
    let mut best_intensity = 0.0f32;
    let mut glow_color = (0, 0, 0);

    for exp in active_explosions {
        let dx = (cx - exp.x) * 0.55;
        let dy = cy - exp.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let glow_radius = exp.radius * 2.2;
        if dist < glow_radius {
            let intensity = (1.0 - dist / glow_radius) * exp.intensity * 0.95;
            if intensity > best_intensity {
                best_intensity = intensity;
                glow_color = exp.color;
            }
        }
    }
    (best_intensity, glow_color)
}

pub fn update_particles_and_excite_stars(
    particles: &mut [Particle],
    stars: &mut [Star],
    delta: f32,
    cols: usize,
    rows: usize,
) {
    let cols_f = cols as f32;
    let rows_f = rows as f32;
    for p in particles {
        p.x += p.vx * delta;
        p.y += p.vy * delta;

        p.vy += 4.5 * delta; // Gravity
        p.vx *= 1.0 - 0.5 * delta; // Drag
        p.vy *= 1.0 - 0.5 * delta;

        p.life -= delta;

        // Excite background stars (ignore smoke particles)
        if p.color != (100, 100, 100) && p.life > 0.0 {
            for star in stars.iter_mut() {
                let sx = star.x * cols_f;
                let sy = star.y * rows_f;
                let dx = p.x - sx;
                let dy = (p.y - sy) * 2.0;
                let dist_sq = dx * dx + dy * dy;
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
}

pub fn blend_horizontal_flare_color(fade: u8, flare_color: (u8, u8, u8)) -> (u8, u8, u8) {
    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
    let fg_b = (fade.saturating_add(45)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
    (fg_r, fg_g, fg_b)
}

pub fn blend_vertical_flare_color(fade: u8, flare_color: (u8, u8, u8)) -> (u8, u8, u8) {
    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
    let fg_b = (fade.saturating_add(30)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
    (fg_r, fg_g, fg_b)
}

pub fn blend_explosion_flare_h_color(current_fg: (u8, u8, u8), fade: u8, color: (u8, u8, u8)) -> (u8, u8, u8) {
    let er = color.0;
    let eg = color.1;
    let eb = color.2;
    (
        current_fg.0.saturating_add((er as f32 * (fade as f32 / 255.0)) as u8),
        current_fg.1.saturating_add((eg as f32 * (fade as f32 / 255.0)) as u8),
        current_fg.2.saturating_add((eb as f32 * (fade as f32 / 255.0) + 40.0 * (fade as f32 / 255.0)) as u8),
    )
}

pub fn blend_explosion_flare_v_color(current_fg: (u8, u8, u8), fade: u8, color: (u8, u8, u8)) -> (u8, u8, u8) {
    let er = color.0;
    let eg = color.1;
    let eb = color.2;
    (
        current_fg.0.saturating_add((er as f32 * (fade as f32 / 255.0)) as u8),
        current_fg.1.saturating_add((eg as f32 * (fade as f32 / 255.0)) as u8),
        current_fg.2.saturating_add((eb as f32 * (fade as f32 / 255.0) + 20.0 * (fade as f32 / 255.0)) as u8),
    )
}

pub fn calculate_star_color_and_sparkle(
    time_elapsed: f32,
    star: &Star,
    lit_color: (u8, u8, u8),
) -> ((u8, u8, u8), f32) {
    let sparkle_base = ((time_elapsed * 2.0 + star.phase).sin() + 1.0) * 0.5;
    let sparkle = (sparkle_base + star.excitation).min(2.0);
    let base_brightness = (sparkle_base * 120.0 + 40.0) as u8;

    let mut r = base_brightness.saturating_add((lit_color.0 as f32 * 0.8) as u8);
    let mut g = base_brightness.saturating_add((lit_color.1 as f32 * 0.8) as u8);
    let mut b = (base_brightness.saturating_add(25)).saturating_add((lit_color.2 as f32 * 0.8) as u8);

    if star.excitation > 0.05 {
        let blend = (star.excitation * 0.7).min(1.0);
        r = (r as f32 * (1.0 - blend) + star.excited_color.0 as f32 * blend).min(255.0) as u8;
        g = (g as f32 * (1.0 - blend) + star.excited_color.1 as f32 * blend).min(255.0) as u8;
        b = (b as f32 * (1.0 - blend) + star.excited_color.2 as f32 * blend).min(255.0) as u8;
    }

    ((r, g, b), sparkle)
}

pub fn calculate_window_glow(
    is_lit_window: bool,
    base_fg: (u8, u8, u8),
    glow_color: (u8, u8, u8),
    best_intensity: f32,
) -> (char, (u8, u8, u8)) {
    if is_lit_window {
        let fg = (
            (base_fg.0 as f32 * (1.0 - best_intensity) + glow_color.0 as f32 * best_intensity).min(255.0) as u8,
            (base_fg.1 as f32 * (1.0 - best_intensity) + glow_color.1 as f32 * best_intensity).min(255.0) as u8,
            (base_fg.2 as f32 * (1.0 - best_intensity) + glow_color.2 as f32 * best_intensity).min(255.0) as u8,
        );
        ('■', fg)
    } else {
        let fg = (
            (glow_color.0 as f32 * best_intensity * 0.38) as u8,
            (glow_color.1 as f32 * best_intensity * 0.38) as u8,
            (glow_color.2 as f32 * best_intensity * 0.38) as u8,
        );
        ('■', fg)
    }
}

pub fn draw_stars(
    stars: &[Star],
    skyline: &[usize],
    active_explosions: &[ActiveExplosion],
    time_elapsed: f32,
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
) {
    let mut flare_candidates: Vec<(usize, f32)> = stars.iter()
        .enumerate()
        .filter(|(_, star)| star.excitation > 0.8)
        .map(|(idx, star)| (idx, star.excitation))
        .collect();
    flare_candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
    let allowed_flares: Vec<usize> = flare_candidates.iter()
        .take(4)
        .map(|&(idx, _)| idx)
        .collect();

    for (i, star) in stars.iter().enumerate() {
        let sx = (star.x * cols as f32) as usize;
        let sy = (star.y * rows as f32) as usize;
        if sx < cols && sy < rows {
            let height_from_bottom = rows.saturating_sub(1).saturating_sub(sy);
            if height_from_bottom >= skyline[sx] {
                let (best_intensity, lit_color) = calculate_star_illumination(
                    sx as f32,
                    sy as f32,
                    active_explosions,
                );

                let (color, sparkle) = calculate_star_color_and_sparkle(
                    time_elapsed,
                    star,
                    lit_color,
                );

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
                    fg: color,
                    bg: (0, 0, 0),
                    bold: final_brightness > 0.6 || star.excitation > 0.3,
                };

                let is_excited = allowed_flares.contains(&i);
                if is_excited {
                    let flare_intensity = ((star.excitation - 0.8) / 0.7 + 0.5).min(1.5);
                    let flare_color = star.excited_color;

                    let h_len = 12;
                    for dx in 1..h_len {
                        let alpha = (120.0 * flare_intensity).max(30.0) as u8;
                        let fade = alpha.saturating_sub((dx * (110 / h_len)) as u8);
                        if fade > 10 {
                            if sx + dx < cols {
                                let cell = &mut grid[sy * cols + (sx + dx)];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy);
                                if h_test >= skyline[sx + dx] && (cell.ch == ' ' || cell.ch == '─') {
                                    cell.ch = '─';
                                    cell.fg = blend_horizontal_flare_color(fade, flare_color);
                                }
                            }
                            if sx >= dx {
                                let cell = &mut grid[sy * cols + (sx - dx)];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy);
                                if h_test >= skyline[sx - dx] && (cell.ch == ' ' || cell.ch == '─') {
                                    cell.ch = '─';
                                    cell.fg = blend_horizontal_flare_color(fade, flare_color);
                                }
                            }
                        }
                    }

                    let v_len = 5;
                    for dy in 1..v_len {
                        let alpha = (90.0 * flare_intensity).max(20.0) as u8;
                        let fade = alpha.saturating_sub((dy * (80 / v_len)) as u8);
                        if fade > 10 {
                            if sy + dy < rows {
                                let cell = &mut grid[(sy + dy) * cols + sx];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy + dy);
                                if h_test >= skyline[sx] && (cell.ch == ' ' || cell.ch == '│') {
                                    cell.ch = '│';
                                    cell.fg = blend_vertical_flare_color(fade, flare_color);
                                }
                            }
                            if sy >= dy {
                                let cell = &mut grid[(sy - dy) * cols + sx];
                                let h_test = rows.saturating_sub(1).saturating_sub(sy - dy);
                                if h_test >= skyline[sx] && (cell.ch == ' ' || cell.ch == '│') {
                                    cell.ch = '│';
                                    cell.fg = blend_vertical_flare_color(fade, flare_color);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn draw_logo(
    logo_text: &str,
    active_explosions: &[ActiveExplosion],
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
) {
    let lines = render_logo_block(logo_text, None);
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
                let (best_intensity, lit_color) = calculate_logo_illumination(
                    gx as f32,
                    gy as f32,
                    active_explosions,
                );

                grid[gy * cols + gx] = TerminalCell {
                    ch,
                    fg: lit_color,
                    bg: (0, 0, 0),
                    bold: best_intensity > 0.1,
                };
            }
        }
    }
}

pub fn draw_skyline(
    skyline: &[usize],
    skyline_windows: &[bool],
    active_explosions: &[ActiveExplosion],
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
) {
    for c in 0..cols {
        let building_h = skyline[c];
        for r in 0..building_h {
            let gy = rows.saturating_sub(1).saturating_sub(r);
            let idx = gy * cols + c;

            let is_lit_window = skyline_windows[idx];
            let base_fg = if is_lit_window {
                (255, 220, 100)
            } else {
                (0, 0, 0)
            };

            let (best_intensity, glow_color) = calculate_window_illumination(
                c as f32,
                gy as f32,
                active_explosions,
            );

            let (ch, fg) = if best_intensity > 0.05 {
                calculate_window_glow(is_lit_window, base_fg, glow_color, best_intensity)
            } else {
                (if is_lit_window { '■' } else { ' ' }, base_fg)
            };

            grid[idx] = TerminalCell {
                ch,
                fg,
                bg: (15, 15, 22),
                bold: best_intensity > 0.2,
            };
        }
    }
}
