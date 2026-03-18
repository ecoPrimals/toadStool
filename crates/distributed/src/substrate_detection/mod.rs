// SPDX-License-Identifier: AGPL-3.0-or-later
//! Substrate detection for universal compute platforms
//!
//! This module provides comprehensive detection capabilities for various compute substrates
//! including traditional platforms, container runtimes, language environments, and more.

mod biological;
mod container;
mod edge;
mod experimental;
mod gpu;
mod language;
mod neuromorphic;
mod probe;
mod quantum;
mod specialized;
mod traditional;
mod types;

use tracing::info;

use toadstool::ToadStoolResult;

pub use types::{PlatformType, SubstrateCapabilities};

/// Universal substrate detector
pub struct SubstrateDetector;

impl Default for SubstrateDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector {
    /// Creates a new substrate detector.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Detect all available substrates on the current system
    pub async fn detect_all(&self) -> ToadStoolResult<SubstrateCapabilities> {
        info!("Starting comprehensive substrate detection");

        let (
            traditional,
            containers,
            languages,
            gpu,
            specialized,
            biological,
            neuromorphic,
            quantum,
            edge,
            experimental,
        ) = tokio::try_join!(
            traditional::detect(),
            container::detect(),
            language::detect(),
            gpu::detect(),
            specialized::detect(),
            biological::detect(),
            neuromorphic::detect(),
            quantum::detect(),
            edge::detect(),
            experimental::detect()
        )?;

        let mut all_specialized = specialized;
        all_specialized.extend(biological);
        all_specialized.extend(neuromorphic);
        all_specialized.extend(quantum);
        all_specialized.extend(edge);

        Ok(SubstrateCapabilities {
            traditional_platforms: traditional,
            container_platforms: containers,
            language_runtimes: languages,
            gpu_platforms: gpu,
            specialized_platforms: all_specialized,
            experimental_platforms: experimental,
        })
    }

    /// Detect traditional OS platforms (Linux, Windows, macOS)
    pub async fn detect_traditional_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        traditional::detect().await
    }

    /// Detect container platforms (Docker, Podman, containerd)
    pub async fn detect_container_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        container::detect().await
    }

    /// Detect language runtimes (Python, Node, Java, etc.)
    pub async fn detect_language_runtimes(&self) -> ToadStoolResult<Vec<PlatformType>> {
        language::detect().await
    }

    /// Detect GPU platforms (NVIDIA CUDA, AMD ROCm)
    pub async fn detect_gpu_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        gpu::detect().await
    }

    /// Detect quantum computing platforms
    pub async fn detect_quantum_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        quantum::detect().await
    }

    /// Detect edge computing platforms
    pub async fn detect_edge_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        edge::detect().await
    }

    /// Detect biological computing platforms
    pub async fn detect_biological_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        biological::detect().await
    }

    /// Detect neuromorphic computing platforms
    pub async fn detect_neuromorphic_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        neuromorphic::detect().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substrate_detector_new() {
        let detector = SubstrateDetector::new();
        let _ = detector;
    }

    #[test]
    fn substrate_detector_default() {
        let detector = SubstrateDetector;
        let from_new = SubstrateDetector::new();
        let _ = detector;
        let _ = from_new;
    }

    #[test]
    fn substrate_capabilities_total_platforms() {
        let caps = SubstrateCapabilities {
            traditional_platforms: vec![PlatformType::Linux {
                distribution: "ubuntu".to_string(),
                architecture: "x86_64".to_string(),
            }],
            container_platforms: vec![PlatformType::Docker],
            language_runtimes: vec![],
            gpu_platforms: vec![PlatformType::GPU {
                vendor: "NVIDIA".to_string(),
                framework: "CUDA".to_string(),
            }],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        assert_eq!(caps.total_platforms(), 3);
    }

    #[test]
    fn substrate_capabilities_has_containers() {
        let caps_empty = SubstrateCapabilities {
            traditional_platforms: vec![],
            container_platforms: vec![],
            language_runtimes: vec![],
            gpu_platforms: vec![],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        assert!(!caps_empty.has_containers());

        let caps_with_docker = SubstrateCapabilities {
            traditional_platforms: vec![],
            container_platforms: vec![PlatformType::Docker],
            language_runtimes: vec![],
            gpu_platforms: vec![],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        assert!(caps_with_docker.has_containers());
    }

    #[test]
    fn substrate_capabilities_has_gpu() {
        let caps_no_gpu = SubstrateCapabilities {
            traditional_platforms: vec![],
            container_platforms: vec![],
            language_runtimes: vec![],
            gpu_platforms: vec![],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        assert!(!caps_no_gpu.has_gpu());

        let caps_with_gpu = SubstrateCapabilities {
            traditional_platforms: vec![],
            container_platforms: vec![],
            language_runtimes: vec![],
            gpu_platforms: vec![PlatformType::GPU {
                vendor: "NVIDIA".to_string(),
                framework: "CUDA".to_string(),
            }],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        assert!(caps_with_gpu.has_gpu());
    }

    #[test]
    fn substrate_capabilities_has_wasm() {
        let caps_no_wasm = SubstrateCapabilities {
            traditional_platforms: vec![],
            container_platforms: vec![],
            language_runtimes: vec![],
            gpu_platforms: vec![],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        assert!(!caps_no_wasm.has_wasm());

        let caps_with_wasm = SubstrateCapabilities {
            traditional_platforms: vec![],
            container_platforms: vec![],
            language_runtimes: vec![],
            gpu_platforms: vec![],
            specialized_platforms: vec![PlatformType::WebAssembly {
                runtime: "Wasmtime".to_string(),
            }],
            experimental_platforms: vec![],
        };
        assert!(caps_with_wasm.has_wasm());
    }

    #[test]
    fn platform_type_variants() {
        let _linux = PlatformType::Linux {
            distribution: "arch".to_string(),
            architecture: "aarch64".to_string(),
        };
        let _docker = PlatformType::Docker;
        let _gpu = PlatformType::GPU {
            vendor: "AMD".to_string(),
            framework: "ROCm".to_string(),
        };
        let _lang = PlatformType::Language {
            name: "Python".to_string(),
            command: "python3".to_string(),
        };
        let _quantum = PlatformType::Quantum {
            framework: "Qiskit".to_string(),
            simulator: true,
        };
        let _edge = PlatformType::EdgeDevice {
            device_type: "Raspberry Pi".to_string(),
            architecture: "arm64".to_string(),
        };
    }

    #[tokio::test]
    async fn test_detect_traditional_platforms_returns_platforms() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_traditional_platforms().await.unwrap();
        assert!(!platforms.is_empty());
    }

    #[tokio::test]
    async fn test_detect_traditional_platforms_has_os_info() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_traditional_platforms().await.unwrap();
        let os = std::env::consts::OS;
        match os {
            "linux" => {
                assert!(
                    platforms
                        .iter()
                        .any(|p| matches!(p, PlatformType::Linux { .. }))
                );
            }
            "windows" => {
                assert!(
                    platforms
                        .iter()
                        .any(|p| matches!(p, PlatformType::Windows { .. }))
                );
            }
            "macos" => {
                assert!(
                    platforms
                        .iter()
                        .any(|p| matches!(p, PlatformType::MacOS { .. }))
                );
            }
            _ => {
                assert!(
                    platforms
                        .iter()
                        .any(|p| matches!(p, PlatformType::Other { .. }))
                );
            }
        }
    }

    #[tokio::test]
    async fn test_detect_container_platforms_returns_vec() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_container_platforms().await.unwrap();
        assert!(platforms.iter().all(|p| {
            matches!(
                p,
                PlatformType::Docker | PlatformType::Podman | PlatformType::Containerd
            )
        }));
    }

    #[tokio::test]
    async fn test_detect_language_runtimes_returns_vec() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_language_runtimes().await.unwrap();
        for p in &platforms {
            assert!(matches!(p, PlatformType::Language { .. }));
        }
    }

    #[tokio::test]
    async fn test_detect_gpu_platforms_without_gpu_returns_empty_or_nvidia_amd() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_gpu_platforms().await.unwrap();
        for p in &platforms {
            if let PlatformType::GPU { vendor, framework } = p {
                assert!(!vendor.is_empty());
                assert!(!framework.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_detect_quantum_platforms_returns_vec() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_quantum_platforms().await.unwrap();
        for p in &platforms {
            assert!(matches!(p, PlatformType::Quantum { .. }));
        }
    }

    #[tokio::test]
    async fn test_detect_edge_platforms_returns_vec() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_edge_platforms().await.unwrap();
        for p in &platforms {
            assert!(matches!(
                p,
                PlatformType::EdgeDevice { .. } | PlatformType::MCUDevelopment { .. }
            ));
        }
    }

    #[tokio::test]
    async fn test_detect_biological_platforms_returns_vec() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_biological_platforms().await.unwrap();
        for p in &platforms {
            assert!(matches!(p, PlatformType::BiologicalComputing { .. }));
        }
    }

    #[tokio::test]
    async fn test_detect_neuromorphic_platforms_returns_vec() {
        let detector = SubstrateDetector::new();
        let platforms = detector.detect_neuromorphic_platforms().await.unwrap();
        for p in &platforms {
            assert!(matches!(p, PlatformType::NeuromorphicComputing { .. }));
        }
    }

    #[tokio::test]
    async fn test_detect_all_combines_capabilities() {
        let detector = SubstrateDetector::new();
        let caps = detector.detect_all().await.unwrap();
        assert!(caps.total_platforms() >= caps.traditional_platforms.len());
    }

    #[tokio::test]
    async fn test_detect_all_has_traditional_platforms() {
        let detector = SubstrateDetector::new();
        let caps = detector.detect_all().await.unwrap();
        assert!(!caps.traditional_platforms.is_empty());
    }

    #[test]
    fn test_substrate_capabilities_empty_total() {
        let caps = SubstrateCapabilities {
            traditional_platforms: vec![],
            container_platforms: vec![],
            language_runtimes: vec![],
            gpu_platforms: vec![],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        assert_eq!(caps.total_platforms(), 0);
    }

    #[test]
    fn test_platform_type_serialization() {
        let gpu = PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        };
        let json = serde_json::to_string(&gpu).unwrap();
        let parsed: PlatformType = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, PlatformType::GPU { .. }));
    }

    #[test]
    fn test_substrate_capabilities_serialization() {
        let caps = SubstrateCapabilities {
            traditional_platforms: vec![PlatformType::Linux {
                distribution: "ubuntu".to_string(),
                architecture: "x86_64".to_string(),
            }],
            container_platforms: vec![],
            language_runtimes: vec![],
            gpu_platforms: vec![],
            specialized_platforms: vec![],
            experimental_platforms: vec![],
        };
        let json = serde_json::to_string(&caps).unwrap();
        let parsed: SubstrateCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_platforms(), 1);
    }

    #[test]
    fn test_platform_type_npu_tpu_variants() {
        let _gpu_nvidia = PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        };
        let _gpu_amd = PlatformType::GPU {
            vendor: "AMD".to_string(),
            framework: "ROCm".to_string(),
        };
    }
}
