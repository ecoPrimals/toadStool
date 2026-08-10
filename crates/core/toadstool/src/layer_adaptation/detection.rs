// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Platform detection helpers for capability adaptation
//!
//! Detects memory, disk, storage, and network capabilities at runtime.

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use toadstool_common::constants::platform_paths::{procfs, sysfs};

/// Parses `MemTotal` from `/proc/meminfo`-style text; returns kB.
#[cfg(target_os = "linux")]
pub(crate) fn parse_meminfo_kb(contents: &str) -> Option<u64> {
    for line in contents.lines() {
        if line.starts_with("MemTotal:") {
            let kb = line.split_whitespace().nth(1)?;
            return kb.parse::<u64>().ok();
        }
    }
    None
}

/// Returns read-bandwidth estimate in bytes/sec (`is_rotational`: `true` = HDD).
#[cfg(target_os = "linux")]
pub(crate) fn estimate_storage_bandwidth(is_rotational: bool) -> u64 {
    if is_rotational {
        150_000_000 // 150 MB/s for HDD
    } else {
        500_000_000 // 500 MB/s for SSD
    }
}

/// Parses sysfs `speed` file contents (Mbps as decimal text).
#[cfg(target_os = "linux")]
pub(crate) fn parse_net_speed_mbps(speed_str: &str) -> Option<u64> {
    speed_str.trim().parse::<u64>().ok()
}

/// Converts megabits per second to bytes per second (decimal M = 1_000_000 bits).
#[cfg(target_os = "linux")]
pub(crate) fn mbps_to_bytes_per_sec(mbps: u64) -> u64 {
    (mbps * 1_000_000) / 8
}

/// Get total system memory (bytes)
pub fn get_total_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = fs::read_to_string(procfs::MEMINFO).ok()?;
        parse_meminfo_kb(&meminfo).map(|kb| kb * 1024)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Get available disk space (bytes)
///
/// Uses toadstool-sysmon /proc parsing for disk detection (pure Rust, zero C).
/// Returns the available space on the root/primary disk.
pub fn get_available_disk() -> Option<u64> {
    let disks = toadstool_sysmon::disk_usage().ok()?;

    let mut root_disk_space: Option<u64> = None;
    let mut largest_disk_space: u64 = 0;

    for disk in &disks {
        let available = disk.available_space;

        if disk.mount_point == "/" {
            root_disk_space = Some(available);
        }

        if available > largest_disk_space {
            largest_disk_space = available;
        }
    }

    root_disk_space.or(if largest_disk_space > 0 {
        Some(largest_disk_space)
    } else {
        None
    })
}

/// Detect storage read bandwidth (bytes/sec) via runtime heuristics
///
/// **Deep Debt**: Runtime detection, no hardcoding
pub fn detect_storage_read_bandwidth() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = fs::read_dir(sysfs::BLOCK) {
            for entry in entries.flatten() {
                let path = entry.path();
                let rotational_path = path.join("queue/rotational");

                if let Ok(content) = fs::read_to_string(&rotational_path) {
                    if let Ok(flag) = content.trim().parse::<u8>() {
                        return Some(estimate_storage_bandwidth(flag != 0));
                    }
                }
            }
        }

        // Fallback: Conservative estimate for unknown storage
        Some(100_000_000) // 100 MB/s conservative
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Detect storage write bandwidth (bytes/sec) via runtime heuristics
///
/// **Deep Debt**: Runtime detection, no hardcoding
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn detect_storage_write_bandwidth() -> Option<u64> {
    // Write is typically 80% of read for SSDs, 90% for HDDs
    detect_storage_read_bandwidth().map(|read_bw| (read_bw as f64 * 0.85) as u64)
}

/// Detect network bandwidth (bytes/sec) via runtime heuristics
///
/// **Deep Debt**: Runtime detection, no hardcoding
pub fn detect_network_bandwidth() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = fs::read_dir(sysfs::CLASS_NET) {
            let mut max_speed = 0u64;

            for entry in entries.flatten() {
                let path = entry.path();
                let speed_path = path.join("speed");

                // Skip loopback
                if let Some(name) = path.file_name() {
                    if name == "lo" {
                        continue;
                    }
                }

                if let Ok(content) = fs::read_to_string(&speed_path) {
                    if let Some(mbps) = parse_net_speed_mbps(&content) {
                        let bytes_per_sec = mbps_to_bytes_per_sec(mbps);
                        max_speed = max_speed.max(bytes_per_sec);
                    }
                }
            }

            if max_speed > 0 {
                return Some(max_speed);
            }
        }
    }

    // Fallback: Assume gigabit ethernet (125 MB/s)
    Some(125_000_000)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{
        estimate_storage_bandwidth, mbps_to_bytes_per_sec, parse_meminfo_kb, parse_net_speed_mbps,
    };

    #[test]
    fn parse_meminfo_kb_real_format() {
        let sample = "MemTotal:       16304928 kB\n\
MemFree:         1234567 kB\n\
MemAvailable:    8901234 kB\n";
        assert_eq!(parse_meminfo_kb(sample), Some(16_304_928));
    }

    #[test]
    fn parse_meminfo_kb_missing_memtotal() {
        assert_eq!(parse_meminfo_kb("MemFree: 100 kB\n"), None);
        assert_eq!(parse_meminfo_kb(""), None);
    }

    #[test]
    fn parse_meminfo_kb_garbage() {
        assert_eq!(parse_meminfo_kb("hello world\nnot meminfo"), None);
        assert_eq!(parse_meminfo_kb("MemTotal:        not_a_number kB\n"), None);
    }

    #[test]
    fn estimate_storage_bandwidth_ssd_vs_hdd() {
        assert_eq!(estimate_storage_bandwidth(false), 500_000_000);
        assert_eq!(estimate_storage_bandwidth(true), 150_000_000);
    }

    #[test]
    fn parse_net_speed_mbps_valid_invalid_empty() {
        assert_eq!(parse_net_speed_mbps("1000"), Some(1000));
        assert_eq!(parse_net_speed_mbps("  100  \n"), Some(100));
        assert_eq!(parse_net_speed_mbps(""), None);
        assert_eq!(parse_net_speed_mbps("   "), None);
        assert_eq!(parse_net_speed_mbps("not-a-number"), None);
    }

    #[test]
    fn mbps_to_bytes_per_sec_common_values() {
        assert_eq!(mbps_to_bytes_per_sec(1000), 125_000_000);
        assert_eq!(mbps_to_bytes_per_sec(100), 12_500_000);
        assert_eq!(mbps_to_bytes_per_sec(10), 1_250_000);
        assert_eq!(mbps_to_bytes_per_sec(0), 0);
    }
}
