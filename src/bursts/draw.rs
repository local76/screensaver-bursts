//! Drawing implementation for Bursts.

use crate::runner::core::TerminalCell;
use super::Bursts;
use super::types::ActiveExplosion;
use super::drawing;
use super::physics;

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
        drawing::draw_stars(
            &self.stars,
            &self.skyline,
            &active_explosions,
            self.time_elapsed,
            grid,
            cols,
            rows,
        );

        // 2. Draw centered logo
        drawing::draw_logo(
            &self.logo_text,
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
        drawing::draw_skyline(
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
