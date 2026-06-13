//! Helper drawing functions for the bursts screensaver.

use crate::runner::core::TerminalCell;
use crate::runner::core::logo_block::render_logo_block;
use super::types::{ActiveExplosion, Star};
use super::physics::{
    calculate_star_illumination, calculate_star_color_and_sparkle,
    blend_horizontal_flare_color, blend_vertical_flare_color,
    calculate_logo_illumination, calculate_window_illumination,
    calculate_window_glow,
};

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
