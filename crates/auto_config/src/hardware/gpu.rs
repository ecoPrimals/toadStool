// SPDX-License-Identifier: AGPL-3.0-only
//! GPU detection and vendor support

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolResult;

use super::HardwareDetector;

/// GPU information and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU model name.
    pub name: String,
    /// Vendor (NVIDIA, AMD, Intel).
    pub vendor: String,
    /// VRAM in GB.
    pub memory_gb: f64,
    /// Driver version string.
    pub driver_version: String,
    /// Compute capability (e.g. CUDA sm_xx).
    pub compute_capability: String,
    /// CUDA support.
    pub supports_cuda: bool,
    /// OpenCL support.
    pub supports_opencl: bool,
}

/// Detect GPU capabilities
pub async fn detect_gpus(_detector: &HardwareDetector) -> ToadStoolResult<Vec<GpuInfo>> {
    let mut gpus = Vec::new();

    // Try to detect NVIDIA GPUs using nvidia-smi
    if let Ok(nvidia_gpus) = detect_nvidia_gpus().await {
        gpus.extend(nvidia_gpus);
    }

    // Try to detect AMD GPUs
    if let Ok(amd_gpus) = detect_amd_gpus().await {
        gpus.extend(amd_gpus);
    }

    // Try to detect Intel GPUs
    gpus.extend(detect_intel_gpus());

    debug!("Detected {} GPU(s)", gpus.len());
    Ok(gpus)
}

/// Parse nvidia-smi CSV output (--format=csv,noheader,nounits).
/// Columns: name, memory.total (MB), `driver_version`
pub(crate) fn parse_nvidia_smi_csv(output: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() >= 3 {
            let name = parts[0].to_string();
            let mem_mb = parts[1].parse::<f64>().unwrap_or(0.0);
            let memory_gb = mem_mb / 1024.0;

            gpus.push(GpuInfo {
                name: name.clone(),
                vendor: "NVIDIA".to_string(),
                memory_gb,
                driver_version: "unknown".to_string(),
                compute_capability: get_nvidia_compute_capability(&name),
                supports_cuda: true,
                supports_opencl: true,
            });
        }
    }

    gpus
}

/// Detect NVIDIA GPUs
async fn detect_nvidia_gpus() -> ToadStoolResult<Vec<GpuInfo>> {
    let gpus = if let Ok(output) = tokio::process::Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total,driver_version")
        .arg("--format=csv,noheader,nounits")
        .output()
        .await
    {
        parse_nvidia_smi_csv(&String::from_utf8_lossy(&output.stdout))
    } else {
        Vec::new()
    };

    Ok(gpus)
}

/// Detect AMD GPUs
async fn detect_amd_gpus() -> ToadStoolResult<Vec<GpuInfo>> {
    let mut gpus = Vec::new();

    // Try to detect AMD GPUs using rocm-smi (if available)
    if let Ok(output) = tokio::process::Command::new("rocm-smi")
        .arg("--showproductname")
        .arg("--showmeminfo")
        .output()
        .await
    {
        // Parse rocm-smi output (simplified)
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("AMD") || output_str.contains("Radeon") {
            gpus.push(GpuInfo {
                name: "AMD GPU".to_string(),
                vendor: "AMD".to_string(),
                memory_gb: 8.0, // Default assumption
                driver_version: "Unknown".to_string(),
                compute_capability: "RDNA".to_string(),
                supports_cuda: false,
                supports_opencl: true,
            });
        }
    }

    Ok(gpus)
}

/// Detect Intel GPUs
fn detect_intel_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    // Intel GPU detection is more complex and platform-dependent
    // For now, we'll do a simple check
    if cfg!(target_os = "linux") && Path::new("/dev/dri").exists() {
        // Assume Intel integrated graphics
        gpus.push(GpuInfo {
            name: "Intel Integrated Graphics".to_string(),
            vendor: "Intel".to_string(),
            memory_gb: 2.0, // Shared system memory
            driver_version: "Unknown".to_string(),
            compute_capability: "Gen9+".to_string(),
            supports_cuda: false,
            supports_opencl: true,
        });
    }

    gpus
}

/// Get NVIDIA compute capability from GPU name
pub(crate) fn get_nvidia_compute_capability(gpu_name: &str) -> String {
    // Simplified mapping of GPU names to compute capabilities
    if gpu_name.contains("RTX 40") || gpu_name.contains("4090") || gpu_name.contains("4080") {
        "8.9".to_string()
    } else if gpu_name.contains("RTX 30") || gpu_name.contains("3090") || gpu_name.contains("3080")
    {
        "8.6".to_string()
    } else if gpu_name.contains("RTX 20")
        || gpu_name.contains("2080")
        || gpu_name.contains("2070")
        || gpu_name.contains("GTX 16")
        || gpu_name.contains("1660")
        || gpu_name.contains("1650")
    {
        "7.5".to_string()
    } else if gpu_name.contains("GTX 10") || gpu_name.contains("1080") || gpu_name.contains("1070")
    {
        "6.1".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Calculate GPU performance score
#[must_use]
pub fn calculate_gpu_score(gpu_info: &[GpuInfo]) -> f64 {
    if gpu_info.is_empty() {
        return 20.0; // Integrated graphics assumption
    }

    let Some(best_gpu) = gpu_info.iter().max_by(|a, b| {
        a.memory_gb
            .partial_cmp(&b.memory_gb)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) else {
        return 20.0; // Fallback to integrated graphics score
    };

    let memory_score = (best_gpu.memory_gb / 24.0 * 50.0).min(50.0);
    let vendor_score = match best_gpu.vendor.as_str() {
        "NVIDIA" => 40.0,
        "AMD" => 35.0,
        "Intel" => 20.0,
        _ => 15.0,
    };
    let compute_score = if best_gpu.supports_cuda { 10.0 } else { 5.0 };

    memory_score + vendor_score + compute_score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_info_serialization() {
        let gpu = GpuInfo {
            name: "NVIDIA GeForce RTX 4090".to_string(),
            vendor: "NVIDIA".to_string(),
            memory_gb: 24.0,
            driver_version: "535.0".to_string(),
            compute_capability: "8.9".to_string(),
            supports_cuda: true,
            supports_opencl: true,
        };

        let json = serde_json::to_string(&gpu).unwrap();
        let deserialized: GpuInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, gpu.name);
        assert_eq!(deserialized.vendor, gpu.vendor);
        assert!((deserialized.memory_gb - gpu.memory_gb).abs() < f64::EPSILON);
    }

    #[test]
    fn test_get_nvidia_compute_capability_rtx40() {
        assert_eq!(
            get_nvidia_compute_capability("NVIDIA GeForce RTX 4090"),
            "8.9"
        );
        assert_eq!(get_nvidia_compute_capability("RTX 4080"), "8.9");
        assert_eq!(get_nvidia_compute_capability("RTX 40 Series"), "8.9");
    }

    #[test]
    fn test_get_nvidia_compute_capability_rtx30() {
        assert_eq!(
            get_nvidia_compute_capability("NVIDIA GeForce RTX 3090"),
            "8.6"
        );
        assert_eq!(get_nvidia_compute_capability("RTX 3080"), "8.6");
    }

    #[test]
    fn test_get_nvidia_compute_capability_rtx20() {
        assert_eq!(get_nvidia_compute_capability("RTX 2080 Ti"), "7.5");
        assert_eq!(get_nvidia_compute_capability("GTX 1660"), "7.5");
        assert_eq!(get_nvidia_compute_capability("GTX 1650"), "7.5");
    }

    #[test]
    fn test_get_nvidia_compute_capability_gtx10() {
        assert_eq!(get_nvidia_compute_capability("GTX 1080"), "6.1");
        assert_eq!(get_nvidia_compute_capability("GTX 1070"), "6.1");
    }

    #[test]
    fn test_get_nvidia_compute_capability_unknown() {
        assert_eq!(get_nvidia_compute_capability("Unknown GPU"), "Unknown");
        assert_eq!(get_nvidia_compute_capability("Quadro K2000"), "Unknown");
    }

    #[test]
    fn test_parse_nvidia_smi_csv_single_gpu() {
        let output = "NVIDIA GeForce RTX 4090, 24576, 535.54.03";
        let gpus = parse_nvidia_smi_csv(output);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 4090");
        assert!((gpus[0].memory_gb - 24.0).abs() < 0.1);
        assert_eq!(gpus[0].vendor, "NVIDIA");
        assert_eq!(gpus[0].compute_capability, "8.9");
        assert!(gpus[0].supports_cuda);
    }

    #[test]
    fn test_parse_nvidia_smi_csv_multiple_gpus() {
        let output = "NVIDIA GeForce RTX 3080, 10240, 535.0\nNVIDIA GeForce RTX 2080, 8192, 535.0";
        let gpus = parse_nvidia_smi_csv(output);
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 3080");
        assert!((gpus[0].memory_gb - 10.0).abs() < 0.1);
        assert_eq!(gpus[1].name, "NVIDIA GeForce RTX 2080");
        assert!((gpus[1].memory_gb - 8.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_nvidia_smi_csv_empty() {
        let gpus = parse_nvidia_smi_csv("");
        assert!(gpus.is_empty());
    }

    #[test]
    fn test_parse_nvidia_smi_csv_invalid_memory() {
        let output = "NVIDIA GeForce RTX 4090, invalid, 535.0";
        let gpus = parse_nvidia_smi_csv(output);
        assert_eq!(gpus.len(), 1);
        assert!((gpus[0].memory_gb - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_nvidia_smi_csv_too_few_columns() {
        let output = "NVIDIA GeForce RTX 4090, 24576";
        let gpus = parse_nvidia_smi_csv(output);
        assert!(gpus.is_empty());
    }

    #[test]
    fn test_calculate_gpu_score_empty() {
        let score = calculate_gpu_score(&[]);
        assert!((score - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_gpu_score_nvidia() {
        let gpus = vec![GpuInfo {
            name: "NVIDIA GeForce RTX 4090".to_string(),
            vendor: "NVIDIA".to_string(),
            memory_gb: 24.0,
            driver_version: "535.0".to_string(),
            compute_capability: "8.9".to_string(),
            supports_cuda: true,
            supports_opencl: true,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 90.0);
    }

    #[test]
    fn test_calculate_gpu_score_amd() {
        let gpus = vec![GpuInfo {
            name: "AMD Radeon RX 7900 XTX".to_string(),
            vendor: "AMD".to_string(),
            memory_gb: 24.0,
            driver_version: "Unknown".to_string(),
            compute_capability: "RDNA".to_string(),
            supports_cuda: false,
            supports_opencl: true,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 80.0);
    }

    #[test]
    fn test_calculate_gpu_score_intel() {
        let gpus = vec![GpuInfo {
            name: "Intel Integrated Graphics".to_string(),
            vendor: "Intel".to_string(),
            memory_gb: 2.0,
            driver_version: "Unknown".to_string(),
            compute_capability: "Gen9+".to_string(),
            supports_cuda: false,
            supports_opencl: true,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 20.0);
        assert!(score < 50.0);
    }

    #[test]
    fn test_calculate_gpu_score_unknown_vendor() {
        let gpus = vec![GpuInfo {
            name: "Unknown GPU".to_string(),
            vendor: "Other".to_string(),
            memory_gb: 8.0,
            driver_version: "Unknown".to_string(),
            compute_capability: "Unknown".to_string(),
            supports_cuda: false,
            supports_opencl: false,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 15.0);
    }

    #[test]
    fn test_calculate_gpu_score_picks_best_by_memory() {
        let gpus = vec![
            GpuInfo {
                name: "NVIDIA GTX 1060".to_string(),
                vendor: "NVIDIA".to_string(),
                memory_gb: 6.0,
                driver_version: "535.0".to_string(),
                compute_capability: "6.1".to_string(),
                supports_cuda: true,
                supports_opencl: true,
            },
            GpuInfo {
                name: "NVIDIA GeForce RTX 4090".to_string(),
                vendor: "NVIDIA".to_string(),
                memory_gb: 24.0,
                driver_version: "535.0".to_string(),
                compute_capability: "8.9".to_string(),
                supports_cuda: true,
                supports_opencl: true,
            },
        ];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 90.0);
    }
}
