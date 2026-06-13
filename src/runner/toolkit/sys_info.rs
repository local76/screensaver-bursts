//! Host system information. Vendored and slimmed from `runner::toolkit::sys_info`.
//!
//! Public API: `get_system_info`, `query_dark_mode`, `query_local_ip`,
//! `query_disk_drives` (delegated to `linux_queries`), `query_current_palette`.

#![allow(dead_code)]

use std::sync::Mutex;
use std::time::{Instant, Duration};
use std::sync::OnceLock;

pub use crate::runner::toolkit::platform::{
    PowerStatus, SystemBiosInfo, DiskDriveInfo, NetworkAdapterInfo, SystemInfo,
};

#[path = "linux_queries.rs"]
mod linux_queries;
#[path = "linux_proc.rs"]
#[cfg(target_os = "linux")]
mod linux_proc;

pub use linux_queries::{query_all_monitors as linux_query_all_monitors, query_disk_drives, query_gpu_names};

static DARK_MODE_CACHE: OnceLock<Mutex<(Option<bool>, Instant)>> = OnceLock::new();
static SYSTEM_INFO_CACHE: OnceLock<Mutex<(Option<SystemInfo>, Instant)>> = OnceLock::new();

/// Returns rich live system info. Cross-platform. Cached for 3 seconds.
pub fn get_system_info() -> SystemInfo {
    let cache_mutex = SYSTEM_INFO_CACHE.get_or_init(|| Mutex::new((None, Instant::now())));
    let mut cache = cache_mutex.lock().unwrap();
    if let Some(ref val) = cache.0 {
        if cache.1.elapsed() < Duration::from_secs(3) {
            return val.clone();
        }
    }
    let val = get_system_info_raw();
    cache.0 = Some(val.clone());
    cache.1 = Instant::now();
    val
}

fn get_system_info_raw() -> SystemInfo {
    let (os, logo_text) = query_os_version();
    let kernel = query_kernel();
    let hostname = query_hostname();
    let cpu = query_cpu_brand().unwrap_or_else(|| "CPU".to_string());
    let (mem_used_mb, mem_total_mb, mem_used_pct) = query_memory();
    let cpu_usage_pct = query_cpu_usage_pct();
    let uptime_secs = query_uptime_secs();
    let power = query_power_status().unwrap_or_default();
    let power_status = if power.ac_online {
        "AC".to_string()
    } else {
        format!("{}% (Battery)", power.battery_percent)
    };
    let disks = query_disk_drives();
    let disk_summary = if let Some(d) = disks.first() {
        format!("{} ~{}G free", d.path, d.free_bytes / (1024 * 1024 * 1024))
    } else {
        "disks".to_string()
    };
    let gpus = query_gpu_names().join(", ");
    let gpus = if gpus.is_empty() { "GPU(s)".to_string() } else { gpus };
    let monitors = format!("{} monitor(s)", linux_query_all_monitors().len());

    SystemInfo {
        os,
        logo_text,
        kernel,
        hostname,
        cpu,
        uptime_secs,
        mem_used_mb,
        mem_total_mb,
        mem_used_pct,
        cpu_usage_pct,
        power_status,
        disk_summary,
        gpus,
        monitors,
    }
}

// ---------------------------------------------------------------------------
// Cross-platform query dispatchers
// ---------------------------------------------------------------------------

fn query_os_version() -> (String, String) {
    #[cfg(target_os = "linux")]
    { linux_proc::query_os_version_linux() }
    #[cfg(target_os = "windows")]
    { ("Windows".to_string(), "WIN11".to_string()) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    { ("Unknown OS".to_string(), "SYS".to_string()) }
}

fn query_kernel() -> String {
    #[cfg(target_os = "linux")]
    { linux_proc::query_kernel_linux() }
    #[cfg(target_os = "windows")]
    { std::env::var("OS").unwrap_or_else(|_| "Windows".to_string()) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    { "unknown".to_string() }
}

fn query_hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "localhost".to_string())
}

fn query_cpu_brand() -> Option<String> {
    #[cfg(target_os = "linux")]
    { linux_proc::query_cpu_brand_linux() }
    #[cfg(not(target_os = "linux"))]
    { None }
}

fn query_memory() -> (u64, u64, f32) {
    #[cfg(target_os = "linux")]
    { linux_proc::query_memory_linux() }
    #[cfg(not(target_os = "linux"))]
    { (0, 0, 0.0) }
}

fn query_cpu_usage_pct() -> f32 {
    #[cfg(target_os = "linux")]
    { linux_proc::query_cpu_usage_pct_linux() }
    #[cfg(not(target_os = "linux"))]
    { 0.0 }
}

fn query_uptime_secs() -> u64 {
    #[cfg(target_os = "linux")]
    { linux_proc::query_uptime_secs_linux() }
    #[cfg(not(target_os = "linux"))]
    { 0 }
}

/// Detect dark mode preference. Cached for 3 seconds.
pub fn query_dark_mode() -> bool {
    let cache_mutex = DARK_MODE_CACHE.get_or_init(|| Mutex::new((None, Instant::now())));
    let mut cache = cache_mutex.lock().unwrap();
    if let Some(val) = cache.0 {
        if cache.1.elapsed() < Duration::from_secs(3) {
            return val;
        }
    }
    let val = query_dark_mode_raw();
    cache.0 = Some(val);
    cache.1 = Instant::now();
    val
}

fn query_dark_mode_raw() -> bool {
    #[cfg(target_os = "linux")]
    { linux_proc::query_dark_mode_linux() }
    #[cfg(not(target_os = "linux"))]
    { true }
}

/// Power status: AC online + battery percent.
pub fn query_power_status() -> Option<PowerStatus> {
    #[cfg(target_os = "linux")]
    { linux_proc::query_power_status_linux() }
    #[cfg(not(target_os = "linux"))]
    { None }
}

/// Find the host's primary outbound IP by opening a UDP socket to 8.8.8.8.
pub fn query_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

/// Query the system palette from accent + dark mode.
pub fn query_current_palette() -> crate::runner::core::screen_palette::ScreenPalette {
    let dark = query_dark_mode();
    crate::runner::core::screen_palette::ScreenPalette::from_system((0, 245, 255), dark)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTheme {
    pub is_dark_mode: bool,
    pub is_high_contrast: bool,
    pub accent_color: (u8, u8, u8),
}

pub fn query_system_theme() -> SystemTheme {
    SystemTheme {
        is_dark_mode: query_dark_mode(),
        is_high_contrast: false,
        accent_color: (0, 245, 255),
    }
}
