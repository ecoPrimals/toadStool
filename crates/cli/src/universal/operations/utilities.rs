// SPDX-License-Identifier: AGPL-3.0-only
//! Utility Operations
//!
//! Extension trait for utility helper methods.

use crate::universal::types::{GpuInfo, HardwareInfo};
use crate::Result;
use std::collections::HashMap;
use std::future::Future;
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
            PlatformType::Other { os, architecture } => format!("other_{os}_{architecture}"),
            PlatformType::EdgeDevice {
                device_type,
                architecture,
            } => {
                format!("edge_{device_type}_{architecture}")
            }
            PlatformType::MCUDevelopment { platform, tool } => format!("mcu_{platform}_{tool}"),
            PlatformType::BiologicalComputing {
                platform,
                simulation,
            } => {
                format!("bio_{platform}_{simulation}")
            }
            PlatformType::Quantum {
                framework,
                simulator,
            } => {
                format!("quantum_{framework}_{simulator}")
            }
            PlatformType::NeuromorphicComputing { platform, hardware } => {
                format!("neuro_{platform}_{hardware}")
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

    #[allow(clippy::cast_precision_loss)]
    async fn get_system_hardware_info(&self) -> Result<HardwareInfo> {
        let cpu_model = toadstool_sysmon::cpu_brand().unwrap_or_else(|_| "Unknown CPU".to_string());
        let memory_gb = toadstool_sysmon::memory_info()
            .map(|m| m.total as f64 / 1024.0 / 1024.0 / 1024.0)
            .unwrap_or(0.0);
        #[allow(clippy::cast_possible_truncation)]
        let cpu_cores = toadstool_sysmon::cpu_count() as u32;

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
            cpu_cores,
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

        Err(crate::CliError::Other("No GPU detected".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_distributed::substrate_detection::PlatformType;

    #[tokio::test]
    async fn test_get_platform_id_linux() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Linux {
            distribution: "Ubuntu 22.04".to_string(),
            architecture: "x86_64".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "linux_ubuntu_22.04_x86_64");
    }

    #[tokio::test]
    async fn test_get_platform_id_docker() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Docker;
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "docker");
    }

    #[tokio::test]
    async fn test_get_platform_id_gpu() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "gpu_nvidia_cuda");
    }

    #[tokio::test]
    async fn test_get_platform_id_wasm() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::WebAssembly {
            runtime: "Wasmtime".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "wasm_wasmtime");
    }

    #[tokio::test]
    async fn test_get_platform_id_language() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Language {
            name: "Python".to_string(),
            command: "python3".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "lang_python");
    }

    #[tokio::test]
    async fn test_get_platform_id_macos() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::MacOS {
            version: "14.0".to_string(),
            architecture: "arm64".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "macos_14_0_arm64");
    }

    #[tokio::test]
    async fn test_get_platform_id_windows() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Windows {
            version: "11".to_string(),
            architecture: "x86_64".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "windows_11_x86_64");
    }

    #[tokio::test]
    async fn test_get_platform_id_other() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Other {
            os: "FreeBSD".to_string(),
            architecture: "amd64".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "other_FreeBSD_amd64");
    }

    #[tokio::test]
    async fn test_get_platform_id_edge_device() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::EdgeDevice {
            device_type: "Raspberry Pi".to_string(),
            architecture: "armv7l".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "edge_Raspberry Pi_armv7l");
    }

    #[tokio::test]
    async fn test_get_platform_metadata_linux() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Linux {
            distribution: "Debian".to_string(),
            architecture: "aarch64".to_string(),
        };
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type"), Some(&"linux".to_string()));
        assert_eq!(meta.get("distribution"), Some(&"Debian".to_string()));
        assert_eq!(meta.get("architecture"), Some(&"aarch64".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_docker() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Docker;
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type"), Some(&"container".to_string()));
        assert_eq!(meta.get("runtime"), Some(&"docker".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_gpu() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::GPU {
            vendor: "AMD".to_string(),
            framework: "ROCm".to_string(),
        };
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type"), Some(&"gpu".to_string()));
        assert_eq!(meta.get("vendor"), Some(&"AMD".to_string()));
        assert_eq!(meta.get("framework"), Some(&"ROCm".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_id_podman() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let id = manager.get_platform_id(&PlatformType::Podman);
        assert_eq!(id, "podman");
    }

    #[tokio::test]
    async fn test_get_platform_id_containerd() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let id = manager.get_platform_id(&PlatformType::Containerd);
        assert_eq!(id, "containerd");
    }

    #[tokio::test]
    async fn test_get_platform_metadata_podman() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let meta = manager.get_platform_metadata(&PlatformType::Podman);
        assert_eq!(meta.get("type"), Some(&"container".to_string()));
        assert_eq!(meta.get("runtime"), Some(&"podman".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_containerd() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let meta = manager.get_platform_metadata(&PlatformType::Containerd);
        assert_eq!(meta.get("type"), Some(&"container".to_string()));
        assert_eq!(meta.get("runtime"), Some(&"containerd".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_wasm() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::WebAssembly {
            runtime: "Wasmtime".to_string(),
        };
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type"), Some(&"wasm".to_string()));
        assert_eq!(meta.get("runtime"), Some(&"Wasmtime".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_language() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Language {
            name: "Python".to_string(),
            command: "python3".to_string(),
        };
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type"), Some(&"language".to_string()));
        assert_eq!(meta.get("name"), Some(&"Python".to_string()));
        assert_eq!(meta.get("command"), Some(&"python3".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_other() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Other {
            os: "FreeBSD".to_string(),
            architecture: "amd64".to_string(),
        };
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type"), Some(&"other".to_string()));
        assert_eq!(meta.get("os"), Some(&"FreeBSD".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_edge_device() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::EdgeDevice {
            device_type: "Raspberry Pi".to_string(),
            architecture: "armv7l".to_string(),
        };
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type"), Some(&"edge_device".to_string()));
        assert_eq!(meta.get("device_type"), Some(&"Raspberry Pi".to_string()));
    }

    #[tokio::test]
    async fn test_get_platform_id_mcu() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::MCUDevelopment {
            platform: "ESP32".to_string(),
            tool: "idf".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "mcu_ESP32_idf");
    }

    #[tokio::test]
    async fn test_get_platform_id_biological() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::BiologicalComputing {
            platform: "DNA".to_string(),
            simulation: true,
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "bio_DNA_true");
    }

    #[tokio::test]
    async fn test_get_platform_id_quantum() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Quantum {
            framework: "Qiskit".to_string(),
            simulator: true,
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "quantum_Qiskit_true");
    }

    #[tokio::test]
    async fn test_get_platform_id_neuromorphic() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::NeuromorphicComputing {
            platform: "Loihi".to_string(),
            hardware: true,
        };
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "neuro_Loihi_true");
    }

    #[tokio::test]
    async fn test_get_system_hardware_info() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let info = manager.get_system_hardware_info().await;
        assert!(info.is_ok());
        let info = info.unwrap();
        assert!(info.cpu_cores > 0);
        assert!(info.memory_gb > 0.0);
    }
}
