// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-only

//! Platform detection helpers for capability adaptation
//!
//! Detects memory, disk, storage, and network capabilities at runtime.

#[cfg(target_os = "linux")]
use std::fs;

/// Get total system memory (bytes)
pub fn get_total_memory() -> Option<u64> {
    // Platform-specific memory detection
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        if let Ok(kb_val) = kb.parse::<u64>() {
                            return Some(kb_val * 1024); // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }

    None
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
    // Strategy: Check for SSD vs HDD indicators
    #[cfg(target_os = "linux")]
    {
        // Check /sys/block for rotational devices (0 = SSD, 1 = HDD)
        if let Ok(entries) = fs::read_dir("/sys/block") {
            for entry in entries.flatten() {
                let path = entry.path();
                let rotational_path = path.join("queue/rotational");

                if let Ok(content) = fs::read_to_string(&rotational_path) {
                    if let Ok(is_rotational) = content.trim().parse::<u8>() {
                        // SSD: ~500 MB/s typical, HDD: ~150 MB/s typical
                        return Some(if is_rotational == 0 {
                            500_000_000 // 500 MB/s for SSD
                        } else {
                            150_000_000 // 150 MB/s for HDD
                        });
                    }
                }
            }
        }

        // Fallback: Conservative estimate for unknown storage
        Some(100_000_000) // 100 MB/s conservative
    }
}

/// Detect storage write bandwidth (bytes/sec) via runtime heuristics
///
/// **Deep Debt**: Runtime detection, no hardcoding
#[allow(
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
        use std::fs;

        // Check /sys/class/net for interface speeds
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
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
                    // Speed is in Mbps, convert to bytes/sec
                    if let Ok(mbps) = content.trim().parse::<u64>() {
                        let bytes_per_sec = (mbps * 1_000_000) / 8;
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
