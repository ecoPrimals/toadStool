//! CPU detection and capabilities

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::ToadStoolResult;

use super::HardwareDetector;

/// CPU information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub family: u32,
    pub base_frequency_mhz: f64,
    pub max_frequency_mhz: f64,
    pub cache_size_kb: u32,
    pub instruction_sets: Vec<String>,
    pub features: CpuFeatures,
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self {
            model_name: "Unknown CPU".to_string(),
            physical_cores: 4,
            logical_cores: 4,
            family: 0,
            base_frequency_mhz: 2000.0,
            max_frequency_mhz: 3000.0,
            cache_size_kb: 8192,
            instruction_sets: Vec::new(),
            features: CpuFeatures::default(),
        }
    }
}

/// CPU feature flags and capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuFeatures {
    pub supports_avx: bool,
    pub supports_avx2: bool,
    pub supports_sse4_1: bool,
    pub supports_sse4_2: bool,
    pub supports_neon: bool,
    /// RISC-V 'V' vector extension (RVV 1.0).
    /// `true` when the running hart advertises the ISA flag `_v` in `/proc/cpuinfo`.
    pub supports_riscv_v: bool,
}

/// Detect CPU capabilities and characteristics
pub async fn detect_cpu(_detector: &HardwareDetector) -> ToadStoolResult<CpuInfo> {
    let mut cpu_info = CpuInfo::default();

    // Try to read CPU info from /proc/cpuinfo on Linux
    if cfg!(target_os = "linux") {
        if let Ok(cpuinfo) = tokio::fs::read_to_string("/proc/cpuinfo").await {
            cpu_info = parse_linux_cpuinfo(&cpuinfo)?;
        }
    }

    // Try to get CPU info from sysctl on macOS
    #[cfg(target_os = "macos")]
    {
        cpu_info = detect_macos_cpu().await?;
    }

    // Try to get CPU info from WMI on Windows
    #[cfg(target_os = "windows")]
    {
        cpu_info = detect_windows_cpu().await?;
    }

    // Fallback: use std::thread::available_parallelism
    if cpu_info.physical_cores == 0 {
        cpu_info.physical_cores = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        cpu_info.logical_cores = cpu_info.physical_cores;
        cpu_info.model_name = "Unknown CPU".to_string();
        warn!("Could not detect CPU details, using fallback values");
    }

    // Detect CPU features
    cpu_info.features = detect_cpu_features()?;

    debug!(
        "Detected CPU: {} with {} cores",
        cpu_info.model_name, cpu_info.physical_cores
    );
    Ok(cpu_info)
}

/// Parse Linux /proc/cpuinfo
fn parse_linux_cpuinfo(cpuinfo: &str) -> ToadStoolResult<CpuInfo> {
    let mut cpu_info = CpuInfo::default();
    let mut core_count = 0;

    for line in cpuinfo.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "processor" => {
                    core_count += 1;
                }
                "model name" => {
                    if cpu_info.model_name.is_empty() {
                        cpu_info.model_name = value.to_string();
                    }
                }
                "cpu family" => {
                    if let Ok(family) = value.parse::<u32>() {
                        cpu_info.family = family;
                    }
                }
                "cpu MHz" => {
                    if let Ok(mhz) = value.parse::<f64>() {
                        cpu_info.base_frequency_mhz = mhz;
                    }
                }
                "cache size" => {
                    if value.contains("KB") {
                        if let Ok(kb) = value.replace(" KB", "").parse::<u32>() {
                            cpu_info.cache_size_kb = kb;
                        }
                    }
                }
                "flags" | "Features" => {
                    cpu_info.instruction_sets = value
                        .split_whitespace()
                        .map(std::string::ToString::to_string)
                        .collect();
                }
                _ => {}
            }
        }
    }

    cpu_info.logical_cores = core_count;
    cpu_info.physical_cores = core_count; // Simplified - would need more logic for HT detection

    Ok(cpu_info)
}

/// Detect macOS CPU information
#[cfg(target_os = "macos")]
async fn detect_macos_cpu() -> ToadStoolResult<CpuInfo> {
    let mut cpu_info = CpuInfo::default();

    // Use sysctl to get CPU information
    if let Ok(output) = tokio::process::Command::new("sysctl")
        .arg("-n")
        .arg("machdep.cpu.brand_string")
        .output()
        .await
    {
        cpu_info.model_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    }

    if let Ok(output) = tokio::process::Command::new("sysctl")
        .arg("-n")
        .arg("hw.physicalcpu")
        .output()
        .await
    {
        if let Ok(cores) = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<usize>()
        {
            cpu_info.physical_cores = cores;
        }
    }

    if let Ok(output) = tokio::process::Command::new("sysctl")
        .arg("-n")
        .arg("hw.logicalcpu")
        .output()
        .await
    {
        if let Ok(cores) = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<usize>()
        {
            cpu_info.logical_cores = cores;
        }
    }

    Ok(cpu_info)
}

/// Detect Windows CPU information
#[cfg(target_os = "windows")]
async fn detect_windows_cpu() -> ToadStoolResult<CpuInfo> {
    let mut cpu_info = CpuInfo::default();

    // Use WMI to get CPU information (simplified implementation)
    if let Ok(output) = tokio::process::Command::new("wmic")
        .arg("cpu")
        .arg("get")
        .arg("Name,NumberOfCores,NumberOfLogicalProcessors")
        .arg("/format:csv")
        .output()
        .await
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Parse WMI output (simplified)
        for line in output_str.lines().skip(1) {
            if !line.trim().is_empty() {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 4 {
                    cpu_info.model_name = parts[1].trim().to_string();
                    if let Ok(cores) = parts[2].trim().parse::<usize>() {
                        cpu_info.physical_cores = cores;
                    }
                    if let Ok(logical) = parts[3].trim().parse::<usize>() {
                        cpu_info.logical_cores = logical;
                    }
                }
                break; // Just get first CPU for now
            }
        }
    }

    Ok(cpu_info)
}

/// Detect CPU features and instruction sets
///
/// EVOLUTION: Runtime detection on TARGET hardware (not HOST)
/// Enables cross-compilation while maintaining accurate feature detection
/// Deep Debt: Complete implementation, detects on actual deployment hardware
fn detect_cpu_features() -> ToadStoolResult<CpuFeatures> {
    let mut features = CpuFeatures::default();

    // x86_64: Runtime detection on actual x86_64 hardware
    #[cfg(target_arch = "x86_64")]
    {
        features.supports_avx = is_x86_feature_detected!("avx");
        features.supports_avx2 = is_x86_feature_detected!("avx2");
        features.supports_sse4_1 = is_x86_feature_detected!("sse4.1");
        features.supports_sse4_2 = is_x86_feature_detected!("sse4.2");

        debug!(
            "x86_64 CPU features detected: AVX={}, AVX2={}, SSE4.1={}, SSE4.2={}",
            features.supports_avx,
            features.supports_avx2,
            features.supports_sse4_1,
            features.supports_sse4_2
        );
    }

    // ARM64: Runtime detection on actual ARM64 hardware
    #[cfg(target_arch = "aarch64")]
    {
        // Import feature detection for ARM targets
        // This is safe: macro only exists when compiling FOR aarch64
        #[cfg(target_os = "linux")]
        {
            features.supports_neon = std::arch::is_aarch64_feature_detected!("neon");
        }
        #[cfg(not(target_os = "linux"))]
        {
            // On non-Linux ARM (macOS, BSD), NEON is standard in ARMv8
            features.supports_neon = true;
        }

        debug!(
            "ARM64 CPU features detected: NEON={}",
            features.supports_neon
        );
    }

    // RISC-V: Future extension detection
    #[cfg(target_arch = "riscv64")]
    {
        // Probe the 'V' (vector) extension from the ISA string in /proc/cpuinfo.
        // Examples of ISA strings that include V: "rv64imafdc_v", "rva22u64v"
        let has_v = std::fs::read_to_string("/proc/cpuinfo")
            .unwrap_or_default()
            .lines()
            .any(|l| {
                let lower = l.to_ascii_lowercase();
                lower.starts_with("isa") && (lower.contains("_v") || lower.ends_with('v'))
            });
        features.supports_riscv_v = has_v;
        debug!(supports_riscv_v = has_v, "RISC-V CPU features detected");
    }

    Ok(features)
}

/// Calculate CPU performance score
pub fn calculate_cpu_score(cpu_info: &CpuInfo) -> f64 {
    let core_score = (cpu_info.physical_cores as f64 / 16.0 * 40.0).min(40.0);
    let frequency_score = (cpu_info.base_frequency_mhz / 4000.0 * 30.0).min(30.0);
    let features_score = if cpu_info.features.supports_avx2 {
        20.0
    } else if cpu_info.features.supports_avx {
        15.0
    } else {
        10.0
    };
    let cache_score = (f64::from(cpu_info.cache_size_kb) / 32768.0 * 10.0).min(10.0);

    core_score + frequency_score + features_score + cache_score
}
