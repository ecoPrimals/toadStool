// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU detection and capabilities

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::ToadStoolResult;

use super::HardwareDetector;

/// CPU information and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU model name.
    pub model_name: String,
    /// Physical core count.
    pub physical_cores: usize,
    /// Logical core count (with hyperthreading).
    pub logical_cores: usize,
    /// CPU family identifier.
    pub family: u32,
    /// Base frequency in MHz.
    pub base_frequency_mhz: f64,
    /// Max turbo frequency in MHz.
    pub max_frequency_mhz: f64,
    /// Total cache size in KB.
    pub cache_size_kb: u32,
    /// Supported instruction sets (e.g. AVX, SSE4).
    pub instruction_sets: Vec<String>,
    /// CPU feature flags.
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

/// CPU feature flags and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[expect(clippy::struct_excessive_bools, reason = "configuration type")]
pub struct CpuFeatures {
    /// AVX support.
    pub supports_avx: bool,
    /// AVX2 support.
    pub supports_avx2: bool,
    /// SSE4.1 support.
    pub supports_sse4_1: bool,
    /// SSE4.2 support.
    pub supports_sse4_2: bool,
    /// ARM NEON support.
    pub supports_neon: bool,
    /// RISC-V 'V' vector extension (RVV 1.0).
    /// `true` when the running hart advertises the ISA flag `_v` in `/proc/cpuinfo`.
    pub supports_riscv_v: bool,
}

/// Detect CPU capabilities and characteristics
pub async fn detect_cpu(_detector: &HardwareDetector) -> ToadStoolResult<CpuInfo> {
    let mut cpu_info = if cfg!(target_os = "linux")
        && let Ok(cpuinfo) = tokio::fs::read_to_string("/proc/cpuinfo").await
    {
        parse_linux_cpuinfo(&cpuinfo)
    } else {
        CpuInfo::default()
    };

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
    cpu_info.features = detect_cpu_features();

    debug!(
        "Detected CPU: {} with {} cores",
        cpu_info.model_name, cpu_info.physical_cores
    );
    Ok(cpu_info)
}

/// Parse Linux /proc/cpuinfo
fn parse_linux_cpuinfo(cpuinfo: &str) -> CpuInfo {
    let mut parsed = CpuInfo {
        model_name: String::new(),
        ..CpuInfo::default()
    };
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
                    if parsed.model_name.is_empty() {
                        parsed.model_name = value.to_string();
                    }
                }
                "cpu family" => {
                    if let Ok(family) = value.parse::<u32>() {
                        parsed.family = family;
                    }
                }
                "cpu MHz" => {
                    if let Ok(mhz) = value.parse::<f64>() {
                        parsed.base_frequency_mhz = mhz;
                    }
                }
                "cache size" => {
                    if value.contains("KB")
                        && let Ok(kb) = value.replace(" KB", "").parse::<u32>()
                    {
                        parsed.cache_size_kb = kb;
                    }
                }
                "flags" | "Features" => {
                    parsed.instruction_sets = value
                        .split_whitespace()
                        .map(std::string::ToString::to_string)
                        .collect();
                }
                _ => {}
            }
        }
    }

    parsed.logical_cores = core_count;
    parsed.physical_cores = core_count; // Simplified - would need more logic for HT detection

    if parsed.model_name.is_empty() {
        parsed.model_name = "Unknown CPU".to_string();
    }

    parsed
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
fn detect_cpu_features() -> CpuFeatures {
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

    features
}

/// Calculate CPU performance score
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info_default() {
        let info = CpuInfo::default();
        assert_eq!(info.model_name, "Unknown CPU");
        assert_eq!(info.physical_cores, 4);
        assert_eq!(info.logical_cores, 4);
        assert_eq!(info.family, 0);
        assert!((info.base_frequency_mhz - 2000.0).abs() < f64::EPSILON);
        assert!((info.max_frequency_mhz - 3000.0).abs() < f64::EPSILON);
        assert_eq!(info.cache_size_kb, 8192);
        assert!(info.instruction_sets.is_empty());
    }

    #[test]
    fn test_cpu_features_default() {
        let features = CpuFeatures::default();
        assert!(!features.supports_avx);
        assert!(!features.supports_avx2);
        assert!(!features.supports_sse4_1);
        assert!(!features.supports_sse4_2);
        assert!(!features.supports_neon);
        assert!(!features.supports_riscv_v);
    }

    #[test]
    fn test_cpu_info_serialization() {
        let info = CpuInfo {
            model_name: "Intel Core i7-9700K".to_string(),
            physical_cores: 8,
            logical_cores: 16,
            family: 6,
            base_frequency_mhz: 3600.0,
            max_frequency_mhz: 4900.0,
            cache_size_kb: 12288,
            instruction_sets: vec!["avx2".to_string(), "sse4_2".to_string()],
            features: CpuFeatures {
                supports_avx: true,
                supports_avx2: true,
                supports_sse4_1: true,
                supports_sse4_2: true,
                supports_neon: false,
                supports_riscv_v: false,
            },
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: CpuInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model_name, info.model_name);
        assert_eq!(deserialized.physical_cores, info.physical_cores);
        assert_eq!(deserialized.logical_cores, info.logical_cores);
        assert_eq!(deserialized.family, info.family);
    }

    #[test]
    fn test_parse_linux_cpuinfo_full() {
        let cpuinfo = r"processor	: 0
vendor_id	: GenuineIntel
cpu family	: 6
model name	: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz
cpu MHz		: 3600.000
cache size	: 12288 KB
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx rdtscp lm constant_tsc rep_good nopl xtopology nonstop_tsc cpuid pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 movbe popcnt aes xsave avx f16c rdrand lahf_lm abm 3dnowprefetch invpcid_single fsgsbase tsc_adjust bmi1 avx2 smep bmi2 invpcid mpx rdseed adx smap clflushopt xsaveopt xsavec xgetbv1 xsaves
processor	: 1
model name	: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz
";

        let result = super::parse_linux_cpuinfo(cpuinfo);
        assert_eq!(
            result.model_name,
            "Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz"
        );
        assert_eq!(result.physical_cores, 2);
        assert_eq!(result.logical_cores, 2);
        assert_eq!(result.family, 6);
        assert!((result.base_frequency_mhz - 3600.0).abs() < f64::EPSILON);
        assert_eq!(result.cache_size_kb, 12288);
        assert!(result.instruction_sets.contains(&"avx2".to_string()));
        assert!(result.instruction_sets.contains(&"sse4_1".to_string()));
    }

    #[test]
    fn test_parse_linux_cpuinfo_empty() {
        let result = super::parse_linux_cpuinfo("");
        assert_eq!(result.model_name, "Unknown CPU");
        assert_eq!(result.physical_cores, 0);
        assert_eq!(result.logical_cores, 0);
    }

    #[test]
    fn test_parse_linux_cpuinfo_arm_features() {
        let cpuinfo = r"processor	: 0
Features	: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp
model name	: ARMv8 Processor
";

        let result = super::parse_linux_cpuinfo(cpuinfo);
        assert_eq!(result.model_name, "ARMv8 Processor");
        assert_eq!(result.logical_cores, 1);
        assert!(result.instruction_sets.contains(&"asimd".to_string()));
    }

    #[test]
    fn test_parse_linux_cpuinfo_malformed_values() {
        let cpuinfo = r"processor	: 0
cpu family	: not_a_number
cpu MHz		: invalid
cache size	: 8192 KB
model name	: Test CPU
";

        let result = super::parse_linux_cpuinfo(cpuinfo);
        assert_eq!(result.model_name, "Test CPU");
        assert_eq!(result.family, 0);
        assert_eq!(result.cache_size_kb, 8192);
    }

    #[test]
    fn test_parse_linux_cpuinfo_cache_without_kb() {
        let cpuinfo = r"processor	: 0
cache size	: 8192 MB
model name	: Test CPU
";

        let result = super::parse_linux_cpuinfo(cpuinfo);
        assert_eq!(result.cache_size_kb, 8192);
    }

    #[test]
    fn test_calculate_cpu_score_basic() {
        let info = CpuInfo::default();
        let score = calculate_cpu_score(&info);
        assert!(score > 0.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn test_calculate_cpu_score_high_end() {
        let info = CpuInfo {
            model_name: "High-end CPU".to_string(),
            physical_cores: 32,
            logical_cores: 64,
            family: 6,
            base_frequency_mhz: 4000.0,
            max_frequency_mhz: 5000.0,
            cache_size_kb: 32768,
            instruction_sets: Vec::new(),
            features: CpuFeatures {
                supports_avx: true,
                supports_avx2: true,
                supports_sse4_1: true,
                supports_sse4_2: true,
                supports_neon: false,
                supports_riscv_v: false,
            },
        };
        let score = calculate_cpu_score(&info);
        assert!(score >= 80.0);
    }

    #[test]
    fn test_calculate_cpu_score_low_end() {
        let info = CpuInfo {
            model_name: "Low-end CPU".to_string(),
            physical_cores: 2,
            logical_cores: 2,
            family: 0,
            base_frequency_mhz: 1000.0,
            max_frequency_mhz: 1500.0,
            cache_size_kb: 1024,
            instruction_sets: Vec::new(),
            features: CpuFeatures::default(),
        };
        let score = calculate_cpu_score(&info);
        assert!(score < 50.0);
    }
}
