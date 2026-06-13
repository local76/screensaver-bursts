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

#[cfg(target_os = "windows")]
mod win_theme_query {
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmGetColorizationColor(pcrColorization: *mut u32, pfOpaqueBlend: *mut i32) -> i32;
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hKey: isize,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut isize,
        ) -> i32;
        fn RegQueryValueExW(
            hKey: isize,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;
            fn RegCloseKey(hKey: isize) -> i32;
    }

    const HKEY_CURRENT_USER: isize = -2147483647; // 0x80000001
    const KEY_QUERY_VALUE: u32 = 1;

    pub fn get_windows_accent() -> Option<(u8, u8, u8)> {
        let mut color: u32 = 0;
        let mut opaque: i32 = 0;
        let hr = unsafe { DwmGetColorizationColor(&mut color, &mut opaque) };
        if hr == 0 {
            Some((
                ((color >> 16) & 0xFF) as u8,
                ((color >> 8) & 0xFF) as u8,
                (color & 0xFF) as u8,
            ))
        } else {
            None
        }
    }

    pub fn get_windows_dark_mode() -> Option<bool> {
        let mut hkcu_key: isize = 0;
        let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
            .encode_utf16()
            .collect();
        let name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();

        let res = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut hkcu_key,
            )
        };
        if res == 0 {
            let mut val: u32 = 0;
            let mut val_type: u32 = 0;
            let mut size = std::mem::size_of::<u32>() as u32;
            let res_query = unsafe {
                RegQueryValueExW(
                    hkcu_key,
                    name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut val_type,
                    &mut val as *mut u32 as *mut u8,
                    &mut size,
                )
            };
            unsafe { RegCloseKey(hkcu_key); }
            if res_query == 0 && val_type == 4 {
                return Some(val == 0);
            }
        }
        None
    }
}

fn get_global_theme_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|appdata| {
            std::path::PathBuf::from(appdata)
                .join("local76")
                .join("theme.yaml")
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HOME").ok().map(|home| {
                    std::path::PathBuf::from(home).join(".config")
                })
            })
            .map(|b| b.join("local76").join("theme.yaml"))
    }
}

static GLOBAL_THEME_CACHE: std::sync::OnceLock<std::sync::Mutex<(Option<(Option<(u8, u8, u8)>, Option<bool>)>, std::time::Instant)>> = std::sync::OnceLock::new();

pub fn load_global_theme() -> (Option<(u8, u8, u8)>, Option<bool>) {
    let cache_mutex = GLOBAL_THEME_CACHE.get_or_init(|| std::sync::Mutex::new((None, std::time::Instant::now())));
    let mut cache = cache_mutex.lock().unwrap();
    if let Some(ref val) = cache.0 {
        if cache.1.elapsed() < std::time::Duration::from_secs(1) {
            return val.clone();
        }
    }
    let val = load_global_theme_raw();
    cache.0 = Some(val.clone());
    cache.1 = std::time::Instant::now();
    val
}

fn load_global_theme_raw() -> (Option<(u8, u8, u8)>, Option<bool>) {
    if let Some(path) = get_global_theme_path() {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut accent = None;
            let mut dark = None;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(idx) = line.find(':') {
                    let key = line[..idx].trim();
                    let val = line[idx + 1..].trim().trim_matches('"').trim_matches('\'');
                    match key {
                        "accent_color" => {
                            if !val.is_empty() && val != "none" {
                                if val.starts_with('#') && val.len() == 7 {
                                    let r = u8::from_str_radix(&val[1..3], 16).unwrap_or(0);
                                    let g = u8::from_str_radix(&val[3..5], 16).unwrap_or(245);
                                    let b = u8::from_str_radix(&val[5..7], 16).unwrap_or(255);
                                    accent = Some((r, g, b));
                                }
                            }
                        }
                        "dark_mode" | "is_dark_mode" => {
                            if let Ok(b) = val.parse::<bool>() {
                                dark = Some(b);
                            }
                        }
                        _ => {}
                    }
                }
            }
            return (accent, dark);
        }
    }
    (None, None)
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
    if let (_, Some(dark)) = load_global_theme() {
        return dark;
    }
    #[cfg(target_os = "linux")]
    { linux_proc::query_dark_mode_linux() }
    #[cfg(target_os = "windows")]
    { win_theme_query::get_windows_dark_mode().unwrap_or(true) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
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
    let (global_accent, global_dark) = load_global_theme();
    let dark = global_dark.unwrap_or_else(|| query_dark_mode());
    let accent = global_accent.unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        { win_theme_query::get_windows_accent().unwrap_or((0, 245, 255)) }
        #[cfg(not(target_os = "windows"))]
        { (0, 245, 255) }
    });
    crate::runner::core::screen_palette::ScreenPalette::from_system(accent, dark)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTheme {
    pub is_dark_mode: bool,
    pub is_high_contrast: bool,
    pub accent_color: (u8, u8, u8),
}

pub fn query_system_theme() -> SystemTheme {
    let (global_accent, global_dark) = load_global_theme();
    let dark = global_dark.unwrap_or_else(|| query_dark_mode());
    let accent = global_accent.unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        { win_theme_query::get_windows_accent().unwrap_or((0, 245, 255)) }
        #[cfg(not(target_os = "windows"))]
        { (0, 245, 255) }
    });
    SystemTheme {
        is_dark_mode: dark,
        is_high_contrast: false,
        accent_color: accent,
    }
}
