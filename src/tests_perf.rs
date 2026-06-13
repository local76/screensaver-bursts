//! Performance benchmark for bursts screensaver.

use crate::bursts::Bursts;
use crate::runner::core::TerminalCell;
use crate::runner::core::screensaver::Screensaver;
use std::time::{Duration, Instant};

#[test]
fn test_screensaver_performance() {
    let mut bursts = Bursts::new();
    // Prevent slow system info calls in tests
    bursts.sys_refresh_timer = -1000.0;

    let cols = 120;
    let rows = 40;
    let mut grid = vec![TerminalCell::default(); cols * rows];
    let dt = Duration::from_millis(16);

    let start = Instant::now();
    for _ in 0..100 {
        bursts.update(dt, cols, rows);
        bursts.draw(&mut grid, cols, rows);
    }
    let elapsed = start.elapsed();
    println!("Performance test: 100 frames took {:?}", elapsed);
    assert!(
        elapsed < Duration::from_millis(1500),
        "Performance test exceeded budget of 1500ms: {:?}",
        elapsed
    );
}
