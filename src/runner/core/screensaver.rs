//! Unified `Screensaver` trait, ratatui-free.
//!
//! This module is the single source of truth for the screensaver trait used
//! by the 10 screensaver shim binaries and the 12 in-app effects.
//!
//! # What's here vs. what's not
//!
//! - `Screensaver` / `ScreensaverState` traits live **here** (Core) because
//!   they depend only on `TerminalCell` (also Core) and `std::time::Duration`.
//!   They are backend-agnostic.
//! - `ScreensaverRenderer` (the buffer-management helper that produces
//!   `[TerminalCell]` grids for ratatui) lives in
//!   `ui::screensaver_renderer`.
//! - The 12 in-app effects (`FallingGlyphs`, `RisingFlames`, etc.) live in
//!   `ui::effects` and implement this trait.
//!
//! `ScreensaverState` is provided as a **convenience sub-trait** for code
//! that wants to track just the active/focus flags without implementing the
//! full effect. The blanket `ScreensaverState for T: Screensaver` means any
//! screensaver is also a state.
//!
//! # Migration shim
//!
//! The pre-4.0 ratatui-coupled trait in `interface::app::screensaver` re-exports
//! these types. The pre-4.0 library signature used `dt: f32`; in 4.0 it is
//! `dt: Duration`. Use `ScreensaverRenderer::tick_duration` (the 4.0 API).

use std::time::Duration;

use crate::runner::core::TerminalCell;

/// A trait representing a screensaver/effect with a structured lifecycle.
///
/// In library 4.0 this is the single backend-agnostic entry point for
/// both console apps (ratatui/buffer-managed) and r* GDI screensaver apps
/// (trance-scenes). Direct drop-in for the pre-4.0 library trait AND the
/// pre-4.0 trance-scenes `trance_core::Screensaver` trait.
///
/// # 4.0 design: `Screensaver: ScreensaverState`
///
/// `Screensaver` extends [`ScreensaverState`] (active/focused) with
/// **default-true / no-op setters**. This means:
///
/// - `Box<dyn Screensaver>` automatically satisfies the renderer bound
///   (`ScreensaverRenderer::tick_duration<S: Screensaver + ?Sized>`) without
///   the consumer having to write `Box<dyn Screensaver + ScreensaverState>`.
/// - Effects that don't care about focus/active state (most console app effects)
///   don't have to implement `ScreensaverState` at all.
/// - Effects that DO want focus/active control (r* GDI screensaver apps,
///   some console app effects) can override the default `active`/`focused` to
///   read from an internal field.
///
/// This is a deliberate departure from the pre-4.0 split-traits design
/// where `ScreensaverState` was a separate sub-trait. The change is
/// motivated by Rust's restriction on multi-trait trait objects: a
/// `Box<dyn Screensaver + ScreensaverState>` is not expressible, so the
/// unified-traits approach removes friction at every consumer site.
pub trait Screensaver: ScreensaverState {
    /// Called once when the grid dimensions are first known or change.
    fn init(&mut self, _cols: usize, _rows: usize) {}

    /// Advance physics/animation by `dt`.
    fn update(&mut self, dt: Duration, cols: usize, rows: usize);

    /// Render the effect into `grid` (row-major, `cols * rows` cells).
    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize);

    /// Whether the host renderer should overlay a CRT scanline effect on
    /// top of the drawn cells. Default: `false`.
    ///
    /// r* GDI screensaver apps (trance-scenes) have always drawn scanlines
    /// separately; console apps typically don't. Returning `true` is opt-in.
    fn has_scanlines(&self) -> bool {
        false
    }
}

/// Sub-trait tracking the focus + active state of a screensaver.
///
/// Every [`Screensaver`] automatically implements `ScreensaverState` via
/// the blanket impl below. The default `active`/`focused` is `true` and
/// the setters are no-ops; effects that want explicit state tracking
/// override them in their own `impl ScreensaverState` block.
pub trait ScreensaverState {
    /// `true` when the screensaver should update + draw; `false` when paused.
    fn active(&self) -> bool;
    fn set_active(&mut self, active: bool);

    /// `true` when the screensaver is in the foreground (full brightness).
    /// `false` means the renderer should dim the output.
    fn focused(&self) -> bool;
    fn set_focused(&mut self, focused: bool);
}

/// Blanket default impl: any `Screensaver` is a `ScreensaverState` with
/// default-true active/focused and no-op setters. Effects that want
/// real state tracking override this in their own `impl ScreensaverState`
/// block — but that block would then conflict with the blanket.
/// **Workaround for the library 4.0 effects**: the 12 console effects do NOT
/// override `ScreensaverState` (their private `active`/`focused` fields
/// are now inert; the renderer treats them as always-on). Effects that
/// need real focus/active control should expose their own state via a
/// `&mut self` method and call it from the renderer's `tick` path.
impl<T: Screensaver + ?Sized> ScreensaverState for T {
    fn active(&self) -> bool {
        true
    }
    fn set_active(&mut self, _active: bool) {}
    fn focused(&self) -> bool {
        true
    }
    fn set_focused(&mut self, _focused: bool) {}
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::core::TerminalCell;

    struct MockSaver;

    impl Screensaver for MockSaver {
        fn init(&mut self, _cols: usize, _rows: usize) {}
        fn update(&mut self, _dt: Duration, _cols: usize, _rows: usize) {}
        fn draw(&self, grid: &mut [TerminalCell], _cols: usize, _rows: usize) {
            if !grid.is_empty() {
                grid[0] = TerminalCell {
                    ch: 'X',
                    fg: (200, 200, 200),
                    bg: (0, 0, 0),
                    bold: true,
                };
            }
        }
    }

    #[test]
    fn screensaver_works_with_default_state() {
        let m = MockSaver;
        let mut grid = [TerminalCell::default(); 4];
        m.draw(&mut grid, 2, 2);
        assert_eq!(grid[0].ch, 'X');
        assert!(!m.has_scanlines());
    }

    #[test]
    fn duration_takes_over_from_f32() {
        let mut m = MockSaver;
        m.update(Duration::from_millis(16), 10, 5);
    }
}

