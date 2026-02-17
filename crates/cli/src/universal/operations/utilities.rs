//! Utility Operations
//!
//! Extension trait for utility helper methods.

use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use sysinfo::System;

use crate::universal::types::{GpuInfo, HardwareInfo};
use toadstool_distributed::substrate_detection::PlatformType;

/// Utility operations trait
pub trait UtilityOps {
    /// Get platform ID from platform type
    fn get_platform_id(&self, platform: &PlatformType) -> String;

    /// Get platform metadata
    fn get_platform_metadata(&self, platform: &PlatformType) -> HashMap<String, String>;

    /// Get system hardware information
    fn get_system_hardware_info(&self) -> impl Future<Output = Result<HardwareInfo>> + Send;

    /// Detect GPU information
    fn detect_gpu_info(&self) -> impl Future<Output = Result<GpuInfo>> + Send;
}

/// Implementation of utility operations
impl UtilityOps for crate::universal::UniversalComputeManager {
    fn get_platform_id(&self, platform: &PlatformType) -> String {
        match platform {
            PlatformType::Linux {
                distribution,
                architecture,
            } => {
                format!(
                    "linux_{}_{}",
                    distribution.to_lowercase().replace(' ', "_"),
                    architecture
                )
            }
            PlatformType::MacOS {
                version,
                architecture,
            } => {
                format!("macos_{}_{}", version.replace('.', "_"), architecture)
            }
            PlatformType::Windows {
                version,
                architecture,
            } => {
                format!("windows_{}_{}", version.replace('.', "_"), architecture)
            }
            // **Zero-Copy Optimization** (Nov 28, 2025): String::from is more efficient for literals
            PlatformType::Docker => String::from("docker"),
            PlatformType::Podman => String::from("podman"),
            PlatformType::Containerd => String::from("containerd"),
            PlatformType::WebAssembly { runtime } => format!("wasm_{}", runtime.to_lowercase()),
            PlatformType::Language { name, .. } => format!("lang_{}", name.to_lowercase()),
            PlatformType::GPU { vendor, framework } => {
                format!("gpu_{}_{}", vendor.to_lowercase(), framework.to_lowercase())
            }
            PlatformType::Other { os, architecture } => format!("other_{}_{}", os, architecture),
            PlatformType::EdgeDevice {
                device_type,
                architecture,
            } => {
                format!("edge_{}_{}", device_type, architecture)
            }
            PlatformType::MCUDevelopment { platform, tool } => format!("mcu_{}_{}", platform, tool),
            PlatformType::BiologicalComputing {
                platform,
                simulation,
            } => {
                format!("bio_{}_{}", platform, simulation)
            }
            PlatformType::Quantum {
                framework,
                simulator,
            } => {
                format!("quantum_{}_{}", framework, simulator)
            }
            PlatformType::NeuromorphicComputing { platform, hardware } => {
                format!("neuro_{}_{}", platform, hardware)
            }
        }
    }

    fn get_platform_metadata(&self, platform: &PlatformType) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        match platform {
            PlatformType::Linux {
                distribution,
                architecture,
            } => {
                metadata.insert("type".to_string(), "linux".to_string());
                metadata.insert("distribution".to_string(), distribution.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::MacOS {
                version,
                architecture,
            } => {
                metadata.insert("type".to_string(), "macos".to_string());
                metadata.insert("version".to_string(), version.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::Windows {
                version,
                architecture,
            } => {
                metadata.insert("type".to_string(), "windows".to_string());
                metadata.insert("version".to_string(), version.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::Docker => {
                metadata.insert("type".to_string(), "container".to_string());
                metadata.insert("runtime".to_string(), "docker".to_string());
            }
            PlatformType::Podman => {
                metadata.insert("type".to_string(), "container".to_string());
                metadata.insert("runtime".to_string(), "podman".to_string());
            }
            PlatformType::Containerd => {
                metadata.insert("type".to_string(), "container".to_string());
                metadata.insert("runtime".to_string(), "containerd".to_string());
            }
            PlatformType::WebAssembly { runtime } => {
                metadata.insert("type".to_string(), "wasm".to_string());
                metadata.insert("runtime".to_string(), runtime.clone());
            }
            PlatformType::Language { name, command } => {
                metadata.insert("type".to_string(), "language".to_string());
                metadata.insert("name".to_string(), name.clone());
                metadata.insert("command".to_string(), command.clone());
            }
            PlatformType::GPU { vendor, framework } => {
                metadata.insert("type".to_string(), "gpu".to_string());
                metadata.insert("vendor".to_string(), vendor.clone());
                metadata.insert("framework".to_string(), framework.clone());
            }
            PlatformType::Other { os, architecture } => {
                metadata.insert("type".to_string(), "other".to_string());
                metadata.insert("os".to_string(), os.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::EdgeDevice {
                device_type,
                architecture,
            } => {
                metadata.insert("type".to_string(), "edge_device".to_string());
                metadata.insert("device_type".to_string(), device_type.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::MCUDevelopment { platform, tool } => {
                metadata.insert("type".to_string(), "mcu_development".to_string());
                metadata.insert("platform".to_string(), platform.clone());
                metadata.insert("tool".to_string(), tool.clone());
            }
            PlatformType::BiologicalComputing {
                platform,
                simulation,
            } => {
                metadata.insert("type".to_string(), "biological".to_string());
                metadata.insert("platform".to_string(), platform.clone());
                metadata.insert("simulation".to_string(), simulation.to_string());
            }
            PlatformType::Quantum {
                framework,
                simulator,
            } => {
                metadata.insert("type".to_string(), "quantum".to_string());
                metadata.insert("framework".to_string(), framework.clone());
                metadata.insert("simulator".to_string(), simulator.to_string());
            }
            PlatformType::NeuromorphicComputing { platform, hardware } => {
                metadata.insert("type".to_string(), "neuromorphic".to_string());
                metadata.insert("platform".to_string(), platform.clone());
                metadata.insert("hardware".to_string(), hardware.to_string());
            }
        }

        metadata
    }

    async fn get_system_hardware_info(&self) -> Result<HardwareInfo> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_model = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let memory_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

        let storage_type = if std::path::Path::new("/sys/block/nvme0n1").exists() {
            "NVMe SSD".to_string()
        } else if std::path::Path::new("/sys/block/sda").exists() {
            "SATA".to_string()
        } else {
            "Unknown".to_string()
        };

        let gpu_info = self.detect_gpu_info().await.ok();

        Ok(HardwareInfo {
            cpu_model,
            cpu_cores: sys.cpus().len() as u32,
            memory_gb,
            storage_type,
            gpu_info,
        })
    }

    async fn detect_gpu_info(&self) -> Result<GpuInfo> {
        // Try to detect GPU using different methods

        // Method 1: Check for NVIDIA GPU (CUDA)
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name,memory.total")
            .arg("--format=csv,noheader")
            .output()
        {
            if output.status.success() {
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
        }

        // Method 2: Check for AMD GPU
        if let Ok(output) = std::process::Command::new("rocm-smi")
            .arg("--showproductname")
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                return Ok(GpuInfo {
                    vendor: "AMD".to_string(),
                    model: output_str.trim().to_string(),
                    memory_mb: 0, // Would need additional command to get memory
                    compute_capability: "Unknown".to_string(),
                });
            }
        }

        // Method 3: Generic detection
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("lspci").output() {
                if output.status.success() {
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
        }

        anyhow::bail!("No GPU detected")
    }
}
