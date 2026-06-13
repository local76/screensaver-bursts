use super::*;
use crate::runner::core::screensaver::Screensaver;
use crate::runner::core::TerminalCell;
use std::time::Duration;

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
    // Prevent slow sys_info calls during tests by setting sys_refresh_timer very negative
    bursts.sys_refresh_timer = -1000.0;

    bursts.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    bursts.draw(&mut grid, 80, 24);
    // Completed without panic, at least some initialization should be done
    assert!(!bursts.stars.is_empty());
}
