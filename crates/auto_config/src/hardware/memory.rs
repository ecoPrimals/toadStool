// SPDX-License-Identifier: AGPL-3.0-only
//! Memory detection and configuration

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolResult;

use super::HardwareDetector;

/// Memory information and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_gb: f64,
    pub available_gb: f64,
    pub memory_type: String,
    pub frequency_mhz: u32,
}

impl Default for MemoryInfo {
    fn default() -> Self {
        Self {
            total_gb: 8.0,
            available_gb: 6.0,
            memory_type: "DDR4".to_string(),
            frequency_mhz: 2400,
        }
    }
}

/// Detect memory configuration
pub async fn detect_memory(_detector: &HardwareDetector) -> ToadStoolResult<MemoryInfo> {
    let mut memory_info = MemoryInfo::default();

    // Try to get memory info from /proc/meminfo on Linux
    if cfg!(target_os = "linux")
        && let Ok(meminfo) = tokio::fs::read_to_string("/proc/meminfo").await
    {
        memory_info = parse_linux_meminfo(&meminfo);
    }

    // macOS memory detection
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .await
        {
            if let Ok(bytes) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u64>()
            {
                memory_info.total_gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            }
        }
    }

    // Windows memory detection
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = tokio::process::Command::new("wmic")
            .arg("computersystem")
            .arg("get")
            .arg("TotalPhysicalMemory")
            .arg("/format:csv")
            .output()
            .await
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines().skip(1) {
                if !line.trim().is_empty() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 2 {
                        if let Ok(bytes) = parts[1].trim().parse::<u64>() {
                            memory_info.total_gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                        }
                    }
                    break;
                }
            }
        }
    }

    debug!("Detected memory: {:.1} GB total", memory_info.total_gb);
    Ok(memory_info)
}

/// Parse Linux /proc/meminfo
fn parse_linux_meminfo(meminfo: &str) -> MemoryInfo {
    let mut memory_info = MemoryInfo::default();

    for line in meminfo.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "MemTotal" => {
                    if let Some(kb_str) = value.split_whitespace().next()
                        && let Ok(kb) = kb_str.parse::<u64>()
                    {
                        memory_info.total_gb = (kb * 1024) as f64 / (1024.0 * 1024.0 * 1024.0);
                    }
                }
                "MemAvailable" => {
                    if let Some(kb_str) = value.split_whitespace().next()
                        && let Ok(kb) = kb_str.parse::<u64>()
                    {
                        memory_info.available_gb = (kb * 1024) as f64 / (1024.0 * 1024.0 * 1024.0);
                    }
                }
                _ => {}
            }
        }
    }

    memory_info
}

/// Calculate memory performance score
#[must_use]
pub fn calculate_memory_score(memory_info: &MemoryInfo) -> f64 {
    (memory_info.total_gb / 32.0 * 100.0).min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_info_default() {
        let info = MemoryInfo::default();
        assert!((info.total_gb - 8.0).abs() < f64::EPSILON);
        assert!((info.available_gb - 6.0).abs() < f64::EPSILON);
        assert_eq!(info.memory_type, "DDR4");
        assert_eq!(info.frequency_mhz, 2400);
    }

    #[test]
    fn test_memory_info_serialization() {
        let info = MemoryInfo {
            total_gb: 32.0,
            available_gb: 24.0,
            memory_type: "DDR5".to_string(),
            frequency_mhz: 4800,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: MemoryInfo = serde_json::from_str(&json).unwrap();
        assert!((deserialized.total_gb - info.total_gb).abs() < f64::EPSILON);
        assert!((deserialized.available_gb - info.available_gb).abs() < f64::EPSILON);
        assert_eq!(deserialized.memory_type, info.memory_type);
        assert_eq!(deserialized.frequency_mhz, info.frequency_mhz);
    }

    #[test]
    fn test_parse_linux_meminfo_full() {
        let meminfo = r"MemTotal:       16777216 kB
MemFree:         4194304 kB
MemAvailable:    8388608 kB
Buffers:          524288 kB
Cached:          4194304 kB
SwapTotal:       8388608 kB
SwapFree:        8388608 kB
";

        let result = super::parse_linux_meminfo(meminfo);
        assert!((result.total_gb - 16.0).abs() < 0.1);
        assert!((result.available_gb - 8.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_linux_meminfo_minimal() {
        let meminfo = r"MemTotal:        8388608 kB
MemAvailable:    6291456 kB
";

        let result = super::parse_linux_meminfo(meminfo);
        assert!((result.total_gb - 8.0).abs() < 0.1);
        assert!((result.available_gb - 6.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_linux_meminfo_empty() {
        let result = super::parse_linux_meminfo("");
        assert!((result.total_gb - 8.0).abs() < f64::EPSILON);
        assert!((result.available_gb - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_linux_meminfo_malformed_values() {
        let meminfo = r"MemTotal:       invalid kB
MemAvailable:   not_a_number kB
";

        let result = super::parse_linux_meminfo(meminfo);
        assert!((result.total_gb - 8.0).abs() < f64::EPSILON);
        assert!((result.available_gb - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_linux_meminfo_partial() {
        let meminfo = "MemTotal:       16777216 kB";
        let result = super::parse_linux_meminfo(meminfo);
        assert!((result.total_gb - 16.0).abs() < 0.1);
        assert!((result.available_gb - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_memory_score_basic() {
        let info = MemoryInfo::default();
        let score = calculate_memory_score(&info);
        assert!(score > 0.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn test_calculate_memory_score_high() {
        let info = MemoryInfo {
            total_gb: 64.0,
            available_gb: 48.0,
            memory_type: "DDR5".to_string(),
            frequency_mhz: 4800,
        };
        let score = calculate_memory_score(&info);
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_memory_score_low() {
        let info = MemoryInfo {
            total_gb: 4.0,
            available_gb: 2.0,
            memory_type: "DDR4".to_string(),
            frequency_mhz: 2400,
        };
        let score = calculate_memory_score(&info);
        assert!(score < 20.0);
    }
}
