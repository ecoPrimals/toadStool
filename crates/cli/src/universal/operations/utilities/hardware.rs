// SPDX-License-Identifier: AGPL-3.0-or-later
//! Host CPU/memory/storage probes and best-effort GPU detection.

use crate::Result;
use crate::universal::types::{GpuInfo, HardwareInfo};

/// Collect system hardware summary (CPU, RAM, storage class, optional GPU).
#[expect(
    clippy::cast_precision_loss,
    reason = "precision loss acceptable for this conversion"
)]
pub(crate) async fn system_hardware_info() -> Result<HardwareInfo> {
    let cpu_model = toadstool_sysmon::cpu_brand().unwrap_or_else(|_| "Unknown CPU".to_string());
    let memory_gb = toadstool_sysmon::memory_info()
        .map(|m| m.total as f64 / 1024.0 / 1024.0 / 1024.0)
        .unwrap_or(0.0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )]
    let cpu_cores = toadstool_sysmon::cpu_count() as u32;

    let storage_type = if std::path::Path::new("/sys/block/nvme0n1").exists() {
        "NVMe SSD".to_string()
    } else if std::path::Path::new("/sys/block/sda").exists() {
        "SATA".to_string()
    } else {
        "Unknown".to_string()
    };

    let gpu_info = detect_gpu_info().await.ok();

    Ok(HardwareInfo {
        cpu_model,
        cpu_cores,
        memory_gb,
        storage_type,
        gpu_info,
    })
}

/// Try NVIDIA, AMD, then generic PCI listing to infer a GPU.
pub(crate) async fn detect_gpu_info() -> Result<GpuInfo> {
    // Try to detect GPU using different methods

    // Method 1: Check for NVIDIA GPU (CUDA)
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total")
        .arg("--format=csv,noheader")
        .output()
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = output_str.trim().split(',').collect();
        if parts.len() >= 2 {
            return Ok(GpuInfo {
                vendor: "NVIDIA".to_string(),
                model: parts[0].trim().to_string(),
                memory_mb: parts[1]
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                compute_capability: "Unknown".to_string(),
            });
        }
    }

    // Method 2: Check for AMD GPU
    if let Ok(output) = std::process::Command::new("rocm-smi")
        .arg("--showproductname")
        .output()
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        return Ok(GpuInfo {
            vendor: "AMD".to_string(),
            model: output_str.trim().to_string(),
            memory_mb: 0, // Would need additional command to get memory
            compute_capability: "Unknown".to_string(),
        });
    }

    // Method 3: Generic detection
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = std::process::Command::new("lspci").output()
            && output.status.success()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("VGA") || line.contains("3D") {
                    return Ok(GpuInfo {
                        vendor: "Unknown".to_string(),
                        model: line.to_string(),
                        memory_mb: 0,
                        compute_capability: "Unknown".to_string(),
                    });
                }
            }
        }
    }

    Err(crate::CliError::Other("No GPU detected".to_string()))
}
