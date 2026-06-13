//! Backend-agnostic screen palette for r* pixel-rendered effects (GDI + console).
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).
//!
//! `ScreenPalette` is a non-ratatui-typed bundle of RGB-tuples that describes
//! the canonical color story for a single r* app surface: background,
//! foreground, accent, dim, hot, cool, plus a few semantic channels used
//! across both console dashboards and fullscreen GDI screensavers.
//!
//! In library 4.0 the goals are:
//!
//! - A single source of truth so `helm`, `pulse`, `trance-scenes`, and
//!   future r* apps all derive their visual identity from the same place.
//! - Backend-agnostic: the struct only holds `(u8, u8, u8)` tuples. console apps
//!   can wrap the tuples in `ratatui::style::Color`; GDI apps can use them
//!   directly. No coupling between the two.
//! - Predictable: the same accent + dark-mode always produces the same palette.
//!
//! # Building a palette
//!
//! The typical flow is:
//!
//! 1. Query the system accent and dark-mode flag (via the platform
//!    helpers, e.g. `toolkit::sys_info::query_system_theme`).
//! 2. Pass both into [`ScreenPalette::from_system`] to construct the
//!    canonical 4.0 palette for the apps suite.
//! 3. Re-use the palette fields directly in your rendering code.
//!
//! # See also
//!
//! - `runner::core::hsl_to_rgb` / `rgb_to_hsl` for the math used to
//!   derive `hot` and `cool` from the accent.
//! - `runner::interface::app::effects::dimensions::Palette` for the
//!   console-typed `(u8, u8, u8)` palette used by the canonical 12 effects
//!   (FallingGlyphs, RisingFlames, etc.). A `From<&ScreenPalette>` impl
//!   bridges the two so effects can consume a `ScreenPalette` directly.

use super::{hsl_to_rgb, rgb_to_hsl};


/// The canonical apps 4.0 screen palette.
///
/// All fields are RGB triples so the palette is usable by both ratatui
/// (via `Color::Rgb`) and GDI pixel renderers (via direct `(r, g, b)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenPalette {
    /// Page background (dark mode: near-black; light mode: near-white).
    pub bg: (u8, u8, u8),
    /// Primary foreground text/glyph color (dark mode: near-white; light mode: near-black).
    pub fg: (u8, u8, u8),
    /// The system accent color (e.g. Windows DWM `AccentColor`).
    pub accent: (u8, u8, u8),
    /// A 35% dim of the accent (used for unfocused borders, dimmed chrome).
    pub dim: (u8, u8, u8),
    /// A bright, warm ramp endpoint (e.g. fire/heat effects, hot/cold contrast).
    /// Computed by rotating the accent hue by +30 degrees at the same lightness.
    pub hot: (u8, u8, u8),
    /// A cool ramp endpoint (rotated -120 degrees; used for chill/cold water/snow effects).
    pub cool: (u8, u8, u8),
    /// A neutral mid-gray for chrome separators.
    pub mid: (u8, u8, u8),
    /// Pure white for "high energy" peaks (fire fronts, etc).
    pub peak: (u8, u8, u8),
}

impl Default for ScreenPalette {
    fn default() -> Self {
        // Default to a green accent in dark mode
        Self::from_system((46, 204, 113), true)
    }
}

impl ScreenPalette {
    /// Construct the canonical 4.0 palette from a system accent and dark-mode flag.
    ///
    /// `accent` should be the 0..=255 RGB triple from the OS (DWM on Windows,
    /// XDG accent on Linux). `is_dark_mode` is the OS's light/dark preference.
    pub fn from_system(accent: (u8, u8, u8), is_dark_mode: bool) -> Self {
        if is_dark_mode {
            Self {
                bg: (0, 0, 0),
                fg: (248, 248, 242),
                accent,
                dim: dim_color(accent, 0.35),
                hot: hue_rotated(accent, 30.0, 0.55),
                cool: hue_rotated(accent, -120.0, 0.45),
                mid: (128, 128, 128),
                peak: (255, 255, 255),
            }
        } else {
            Self {
                bg: (252, 252, 250),
                fg: (40, 42, 54),
                accent,
                dim: dim_color(accent, 0.7),
                hot: hue_rotated(accent, 30.0, 0.55),
                cool: hue_rotated(accent, -120.0, 0.45),
                mid: (160, 160, 160),
                peak: (255, 255, 255),
            }
        }
    }

    /// A "high contrast" variant that boosts the accent to full saturation
    /// and uses near-black/near-white extremes. Useful for accessibility
    /// mode and the trance-scenes CRT look.
    pub fn high_contrast(accent: (u8, u8, u8), is_dark_mode: bool) -> Self {
        let mut p = Self::from_system(accent, is_dark_mode);
        if is_dark_mode {
            p.bg = (0, 0, 0);
            p.fg = (255, 255, 255);
        } else {
            p.bg = (255, 255, 255);
            p.fg = (0, 0, 0);
        }
        p
    }

    /// The apps cyan-ecosystem default for fallbacks when the system accent
    /// is unavailable (e.g. in tests, headless contexts, or non-accent themes).
    pub fn default_dark() -> Self {
        Self::from_system((0, 245, 255), true)
    }

    /// Light-mode default for the apps cyan-ecosystem.
    pub fn default_light() -> Self {
        Self::from_system((0, 180, 200), false)
    }
}

/// Scale each channel of `color` by `factor` (0.0..=1.0) — used to build the
/// `dim` channel.
fn dim_color(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
    (
        (color.0 as f32 * factor) as u8,
        (color.1 as f32 * factor) as u8,
        (color.2 as f32 * factor) as u8,
    )
}

/// Rotate the hue of `color` by `delta_deg` degrees and lock the lightness
/// to `target_lightness`. Used to build `hot` and `cool` ramps.
fn hue_rotated(color: (u8, u8, u8), delta_deg: f32, target_lightness: f32) -> (u8, u8, u8) {
    let (h, _s, _l) = rgb_to_hsl(color.0, color.1, color.2);
    let new_h = (h + delta_deg).rem_euclid(360.0);
    hsl_to_rgb(new_h, 0.95, target_lightness)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dark_matches_cyan_ecosystem() {
        let p = ScreenPalette::default_dark();
        assert_eq!(p.accent, (0, 245, 255));
        assert_eq!(p.bg, (0, 0, 0));
        assert_eq!(p.fg, (248, 248, 242));
    }

    #[test]
    fn default_light_passes_through_accent() {
        let p = ScreenPalette::default_light();
        assert_eq!(p.accent, (0, 180, 200));
        assert_eq!(p.bg, (252, 252, 250));
    }

    #[test]
    fn from_system_preserves_accent() {
        let p = ScreenPalette::from_system((100, 200, 50), true);
        assert_eq!(p.accent, (100, 200, 50));
    }

    #[test]
    fn dim_is_scaled_accent() {
        // 0.35 factor: 100 -> 35
        let p = ScreenPalette::from_system((100, 200, 50), true);
        assert_eq!(p.dim, (35, 70, 17));
    }

    #[test]
    fn hot_and_cool_are_distinct_hues() {
        let p = ScreenPalette::from_system((255, 0, 0), true);
        // Pure red accent: hot should be near orange, cool should be far around the wheel
        assert_ne!(p.hot, p.cool);
        assert_ne!(p.hot, p.accent);
    }

    #[test]
    fn high_contrast_extremes() {
        let dark = ScreenPalette::high_contrast((0, 245, 255), true);
        assert_eq!(dark.bg, (0, 0, 0));
        assert_eq!(dark.fg, (255, 255, 255));
        let light = ScreenPalette::high_contrast((0, 245, 255), false);
        assert_eq!(light.bg, (255, 255, 255));
        assert_eq!(light.fg, (0, 0, 0));
    }
}
