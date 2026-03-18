// SPDX-License-Identifier: AGPL-3.0-or-later
//! Utility Operations
//!
//! Extension trait for utility helper methods.

use crate::Result;
use crate::universal::types::{GpuInfo, HardwareInfo};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use toadstool_distributed::substrate_detection::PlatformType;

/// Utility operations trait
pub trait UtilityOps {
    /// Get platform ID from platform type
    fn get_platform_id(&self, platform: &PlatformType) -> String;

    /// Get platform metadata (`Arc<str>` values = zero-copy clone)
    fn get_platform_metadata(&self, platform: &PlatformType) -> HashMap<String, Arc<str>>;

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

    fn get_platform_metadata(&self, platform: &PlatformType) -> HashMap<String, Arc<str>> {
        let mut metadata = HashMap::new();

        match platform {
            PlatformType::Linux {
                distribution,
                architecture,
            } => {
                metadata.insert("type".to_string(), Arc::from("linux"));
                metadata.insert("distribution".to_string(), Arc::from(distribution.as_str()));
                metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
            }
            PlatformType::MacOS {
                version,
                architecture,
            } => {
                metadata.insert("type".to_string(), Arc::from("macos"));
                metadata.insert("version".to_string(), Arc::from(version.as_str()));
                metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
            }
            PlatformType::Windows {
                version,
                architecture,
            } => {
                metadata.insert("type".to_string(), Arc::from("windows"));
                metadata.insert("version".to_string(), Arc::from(version.as_str()));
                metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
            }
            PlatformType::Docker => {
                metadata.insert("type".to_string(), Arc::from("container"));
                metadata.insert("runtime".to_string(), Arc::from("docker"));
            }
            PlatformType::Podman => {
                metadata.insert("type".to_string(), Arc::from("container"));
                metadata.insert("runtime".to_string(), Arc::from("podman"));
            }
            PlatformType::Containerd => {
                metadata.insert("type".to_string(), Arc::from("container"));
                metadata.insert("runtime".to_string(), Arc::from("containerd"));
            }
            PlatformType::WebAssembly { runtime } => {
                metadata.insert("type".to_string(), Arc::from("wasm"));
                metadata.insert("runtime".to_string(), Arc::from(runtime.as_str()));
            }
            PlatformType::Language { name, command } => {
                metadata.insert("type".to_string(), Arc::from("language"));
                metadata.insert("name".to_string(), Arc::from(name.as_str()));
                metadata.insert("command".to_string(), Arc::from(command.as_str()));
            }
            PlatformType::GPU { vendor, framework } => {
                metadata.insert("type".to_string(), Arc::from("gpu"));
                metadata.insert("vendor".to_string(), Arc::from(vendor.as_str()));
                metadata.insert("framework".to_string(), Arc::from(framework.as_str()));
            }
            PlatformType::Other { os, architecture } => {
                metadata.insert("type".to_string(), Arc::from("other"));
                metadata.insert("os".to_string(), Arc::from(os.as_str()));
                metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
            }
            PlatformType::EdgeDevice {
                device_type,
                architecture,
            } => {
                metadata.insert("type".to_string(), Arc::from("edge_device"));
                metadata.insert("device_type".to_string(), Arc::from(device_type.as_str()));
                metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
            }
            PlatformType::MCUDevelopment { platform, tool } => {
                metadata.insert("type".to_string(), Arc::from("mcu_development"));
                metadata.insert("platform".to_string(), Arc::from(platform.as_str()));
                metadata.insert("tool".to_string(), Arc::from(tool.as_str()));
            }
            PlatformType::BiologicalComputing {
                platform,
                simulation,
            } => {
                metadata.insert("type".to_string(), Arc::from("biological"));
                metadata.insert("platform".to_string(), Arc::from(platform.as_str()));
                metadata.insert(
                    "simulation".to_string(),
                    Arc::from(if *simulation { "true" } else { "false" }),
                );
            }
            PlatformType::Quantum {
                framework,
                simulator,
            } => {
                metadata.insert("type".to_string(), Arc::from("quantum"));
                metadata.insert("framework".to_string(), Arc::from(framework.as_str()));
                metadata.insert(
                    "simulator".to_string(),
                    Arc::from(if *simulator { "true" } else { "false" }),
                );
            }
            PlatformType::NeuromorphicComputing { platform, hardware } => {
                metadata.insert("type".to_string(), Arc::from("neuromorphic"));
                metadata.insert("platform".to_string(), Arc::from(platform.as_str()));
                metadata.insert(
                    "hardware".to_string(),
                    Arc::from(if *hardware { "true" } else { "false" }),
                );
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
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("linux"));
        assert_eq!(meta.get("distribution").map(|s| s.as_ref()), Some("Debian"));
        assert_eq!(
            meta.get("architecture").map(|s| s.as_ref()),
            Some("aarch64")
        );
    }

    #[tokio::test]
    async fn test_get_platform_metadata_docker() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let platform = PlatformType::Docker;
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("container"));
        assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("docker"));
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
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("gpu"));
        assert_eq!(meta.get("vendor").map(|s| s.as_ref()), Some("AMD"));
        assert_eq!(meta.get("framework").map(|s| s.as_ref()), Some("ROCm"));
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
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("container"));
        assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("podman"));
    }

    #[tokio::test]
    async fn test_get_platform_metadata_containerd() {
        let manager = crate::universal::UniversalComputeManager::new()
            .await
            .expect("manager should create");
        let meta = manager.get_platform_metadata(&PlatformType::Containerd);
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("container"));
        assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("containerd"));
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
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("wasm"));
        assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("Wasmtime"));
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
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("language"));
        assert_eq!(meta.get("name").map(|s| s.as_ref()), Some("Python"));
        assert_eq!(meta.get("command").map(|s| s.as_ref()), Some("python3"));
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
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("other"));
        assert_eq!(meta.get("os").map(|s| s.as_ref()), Some("FreeBSD"));
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
        assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("edge_device"));
        assert_eq!(
            meta.get("device_type").map(|s| s.as_ref()),
            Some("Raspberry Pi")
        );
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
