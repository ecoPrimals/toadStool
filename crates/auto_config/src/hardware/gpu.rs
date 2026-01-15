//! GPU detection and vendor support

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolResult;

use super::HardwareDetector;

/// GPU information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub memory_gb: f64,
    pub driver_version: String,
    pub compute_capability: String,
    pub supports_cuda: bool,
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
    if let Ok(intel_gpus) = detect_intel_gpus().await {
        gpus.extend(intel_gpus);
    }

    debug!("Detected {} GPU(s)", gpus.len());
    Ok(gpus)
}

/// Detect NVIDIA GPUs
async fn detect_nvidia_gpus() -> ToadStoolResult<Vec<GpuInfo>> {
    let mut gpus = Vec::new();

    if let Ok(output) = tokio::process::Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total,driver_version")
        .arg("--format=csv,noheader,nounits")
        .output()
        .await
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let memory_mb = parts[1].parse::<f64>().unwrap_or(0.0);
                let memory_gb = memory_mb / 1024.0;
                let _driver_version = parts[2].to_string();

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
    }

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
async fn detect_intel_gpus() -> ToadStoolResult<Vec<GpuInfo>> {
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

    Ok(gpus)
}

/// Get NVIDIA compute capability
fn get_nvidia_compute_capability(gpu_name: &str) -> String {
    // Simplified mapping of GPU names to compute capabilities
    if gpu_name.contains("RTX 40") || gpu_name.contains("4090") || gpu_name.contains("4080") {
        "8.9".to_string()
    } else if gpu_name.contains("RTX 30")
        || gpu_name.contains("3090")
        || gpu_name.contains("3080")
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
    } else if gpu_name.contains("GTX 10")
        || gpu_name.contains("1080")
        || gpu_name.contains("1070")
    {
        "6.1".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Calculate GPU performance score
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
