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
    if cfg!(target_os = "linux") {
        if let Ok(meminfo) = tokio::fs::read_to_string("/proc/meminfo").await {
            memory_info = parse_linux_meminfo(&meminfo)?;
        }
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
fn parse_linux_meminfo(meminfo: &str) -> ToadStoolResult<MemoryInfo> {
    let mut memory_info = MemoryInfo::default();

    for line in meminfo.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "MemTotal" => {
                    if let Some(kb_str) = value.split_whitespace().next() {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            memory_info.total_gb = (kb * 1024) as f64 / (1024.0 * 1024.0 * 1024.0);
                        }
                    }
                }
                "MemAvailable" => {
                    if let Some(kb_str) = value.split_whitespace().next() {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            memory_info.available_gb =
                                (kb * 1024) as f64 / (1024.0 * 1024.0 * 1024.0);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(memory_info)
}

/// Calculate memory performance score
pub fn calculate_memory_score(memory_info: &MemoryInfo) -> f64 {
    (memory_info.total_gb / 32.0 * 100.0).min(100.0)
}
