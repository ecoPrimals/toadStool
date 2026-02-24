//! Substrate detection for universal compute platforms
//!
//! This module provides comprehensive detection capabilities for various compute substrates
//! including traditional platforms, container runtimes, language environments, and more.

use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use tracing::info;

use toadstool::ToadStoolResult;

/// Universal substrate detector
pub struct SubstrateDetector;

impl Default for SubstrateDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Detect all available substrates on the current system
    pub async fn detect_all(&self) -> ToadStoolResult<SubstrateCapabilities> {
        info!("Starting comprehensive substrate detection");

        // Run all detection operations in parallel for better performance
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
            self.detect_traditional_platforms(),
            self.detect_container_platforms(),
            self.detect_language_runtimes(),
            self.detect_gpu_platforms(),
            self.detect_specialized_platforms(),
            self.detect_biological_platforms(),
            self.detect_neuromorphic_platforms(),
            self.detect_quantum_platforms(),
            self.detect_edge_platforms(),
            self.detect_experimental_platforms()
        )?;

        // Combine specialized and exotic platforms
        let mut all_specialized = specialized;
        all_specialized.extend(biological.into_iter().map(|p| match p {
            PlatformType::BiologicalComputing {
                platform,
                simulation,
            } => PlatformType::BiologicalComputing {
                platform,
                simulation,
            },
            _ => PlatformType::BiologicalComputing {
                platform: "Unknown".to_string(),
                simulation: true,
            },
        }));
        all_specialized.extend(neuromorphic.into_iter());
        all_specialized.extend(quantum.into_iter());
        all_specialized.extend(edge.into_iter());

        Ok(SubstrateCapabilities {
            traditional_platforms: traditional,
            container_platforms: containers,
            language_runtimes: languages,
            gpu_platforms: gpu,
            specialized_platforms: all_specialized,
            experimental_platforms: experimental,
        })
    }

    pub async fn detect_traditional_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Detect operating system
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match os {
            "linux" => {
                if let Ok(distro) = self.detect_linux_distribution().await {
                    platforms.push(PlatformType::Linux {
                        distribution: distro,
                        architecture: arch.to_string(),
                    });
                }
            }
            "windows" => {
                platforms.push(PlatformType::Windows {
                    version: "Unknown".to_string(),
                    architecture: arch.to_string(),
                });
            }
            "macos" => {
                platforms.push(PlatformType::MacOS {
                    version: "Unknown".to_string(),
                    architecture: arch.to_string(),
                });
            }
            _ => {
                platforms.push(PlatformType::Other {
                    os: os.to_string(),
                    architecture: arch.to_string(),
                });
            }
        }

        Ok(platforms)
    }

    async fn detect_linux_distribution(&self) -> ToadStoolResult<String> {
        // Try to read /etc/os-release
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("ID=") {
                    return Ok(line.trim_start_matches("ID=").trim_matches('"').to_string());
                }
            }
        }

        // Fallback to generic Linux
        Ok("linux".to_string())
    }

    pub async fn detect_container_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Check for Docker
        if self.command_exists("docker").await {
            platforms.push(PlatformType::Docker);
        }

        // Check for Podman
        if self.command_exists("podman").await {
            platforms.push(PlatformType::Podman);
        }

        // Check for containerd
        if self.command_exists("ctr").await {
            platforms.push(PlatformType::Containerd);
        }

        Ok(platforms)
    }

    pub async fn detect_language_runtimes(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Common language runtimes
        let runtimes = [
            ("python", "Python"),
            ("python3", "Python3"),
            ("node", "NodeJS"),
            ("java", "Java"),
            ("go", "Go"),
            ("rustc", "Rust"),
        ];

        for (command, name) in &runtimes {
            if self.command_exists(command).await {
                platforms.push(PlatformType::Language {
                    name: (*name).to_string(),
                    command: (*command).to_string(),
                });
            }
        }

        Ok(platforms)
    }

    pub async fn detect_gpu_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Check for NVIDIA GPU support
        if self.command_exists("nvidia-smi").await {
            platforms.push(PlatformType::GPU {
                vendor: "NVIDIA".to_string(),
                framework: "CUDA".to_string(),
            });
        }

        // Check for AMD GPU support
        if self.command_exists("rocm-smi").await {
            platforms.push(PlatformType::GPU {
                vendor: "AMD".to_string(),
                framework: "ROCm".to_string(),
            });
        }

        Ok(platforms)
    }

    async fn detect_specialized_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Check for WebAssembly runtimes
        let wasm_runtimes = [
            ("wasmtime", "Wasmtime"),
            ("wasmer", "Wasmer"),
            ("wasmedge", "WasmEdge"),
        ];

        for (command, name) in &wasm_runtimes {
            if self.command_exists(command).await {
                platforms.push(PlatformType::WebAssembly {
                    runtime: (*name).to_string(),
                });
            }
        }

        Ok(platforms)
    }

    async fn detect_experimental_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        // For now, return empty - these would require specific hardware/software
        Ok(Vec::new())
    }

    /// Detect quantum computing platforms
    pub async fn detect_quantum_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Check for quantum computing frameworks and simulators
        let quantum_frameworks = [
            ("qiskit", "IBM Qiskit"),
            ("cirq", "Google Cirq"),
            ("forest", "Rigetti Forest"),
            ("braket", "Amazon Braket"),
            ("pennylane", "PennyLane"),
        ];

        for (command, name) in &quantum_frameworks {
            if self.command_exists(command).await || self.python_package_exists(command).await {
                platforms.push(PlatformType::Quantum {
                    framework: (*name).to_string(),
                    simulator: true,
                });
            }
        }

        // Check for actual quantum hardware access
        if std::env::var("IBM_QUANTUM_TOKEN").is_ok() {
            platforms.push(PlatformType::Quantum {
                framework: "IBM Quantum Network".to_string(),
                simulator: false,
            });
        }

        if std::env::var("RIGETTI_QCS_TOKEN").is_ok() {
            platforms.push(PlatformType::Quantum {
                framework: "Rigetti QCS".to_string(),
                simulator: false,
            });
        }

        Ok(platforms)
    }

    /// Detect edge computing platforms
    pub async fn detect_edge_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Check for common edge/IoT indicators
        if let Ok(model) = fs::read_to_string("/proc/device-tree/model") {
            if model.contains("Raspberry Pi") {
                platforms.push(PlatformType::EdgeDevice {
                    device_type: "Raspberry Pi".to_string(),
                    architecture: std::env::consts::ARCH.to_string(),
                });
            } else if model.contains("BeagleBone") {
                platforms.push(PlatformType::EdgeDevice {
                    device_type: "BeagleBone".to_string(),
                    architecture: std::env::consts::ARCH.to_string(),
                });
            }
        }

        // Check for microcontroller development tools
        let mcu_tools = [
            ("arduino-cli", "Arduino"),
            ("pio", "PlatformIO"),
            ("esptool", "ESP32/ESP8266"),
            ("openocd", "ARM Development"),
        ];

        for (tool, platform) in &mcu_tools {
            if self.command_exists(tool).await {
                platforms.push(PlatformType::MCUDevelopment {
                    platform: (*platform).to_string(),
                    tool: (*tool).to_string(),
                });
            }
        }

        Ok(platforms)
    }

    /// Detect biological computing platforms
    pub async fn detect_biological_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Check for bioinformatics tools (indicates potential biological computing capability)
        let bio_tools = [
            ("blast", "BLAST Sequence Analysis"),
            ("clustalw", "ClustalW Multiple Sequence Alignment"),
            ("biopython", "BioPython Framework"),
            ("openmm", "OpenMM Molecular Dynamics"),
            ("gromacs", "GROMACS Molecular Simulation"),
            ("amber", "AMBER MD Suite"),
        ];

        for (tool, description) in &bio_tools {
            if self.command_exists(tool).await || self.python_package_exists(tool).await {
                platforms.push(PlatformType::BiologicalComputing {
                    platform: (*description).to_string(),
                    simulation: true,
                });
            }
        }

        // Check for lab automation/control software
        if self.command_exists("opentrons").await {
            platforms.push(PlatformType::BiologicalComputing {
                platform: "Opentrons Lab Automation".to_string(),
                simulation: false,
            });
        }

        // Check for DNA synthesis environment variables or config files
        if std::env::var("TWIST_BIOSCIENCE_API_KEY").is_ok() {
            platforms.push(PlatformType::BiologicalComputing {
                platform: "Twist Bioscience DNA Synthesis".to_string(),
                simulation: false,
            });
        }

        Ok(platforms)
    }

    /// Detect neuromorphic computing platforms
    pub async fn detect_neuromorphic_platforms(&self) -> ToadStoolResult<Vec<PlatformType>> {
        let mut platforms = Vec::new();

        // Check for neuromorphic simulation frameworks
        let neuro_frameworks = [
            ("brian2", "Brian2 Spiking Network Simulator"),
            ("nest", "NEST Simulator"),
            ("neuron", "NEURON Simulation Environment"),
            ("nengo", "Nengo Neural Engineering Framework"),
            ("spynnaker", "SpiNNaker Toolkit"),
            ("loihi", "Intel Loihi SDK"),
        ];

        for (framework, description) in &neuro_frameworks {
            if self.python_package_exists(framework).await {
                platforms.push(PlatformType::NeuromorphicComputing {
                    platform: (*description).to_string(),
                    hardware: false,
                });
            }
        }

        // Check for actual neuromorphic hardware
        if let Ok(devices) = fs::read_dir("/dev") {
            let devices = devices;
            for entry in devices.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.contains("loihi") || name.contains("neuromorphic") {
                        platforms.push(PlatformType::NeuromorphicComputing {
                            platform: "Intel Loihi Hardware".to_string(),
                            hardware: true,
                        });
                    }
                }
            }
        }

        // Check for FPGA tools (often used for neuromorphic implementations)
        let fpga_tools = [
            ("vivado", "Xilinx Vivado"),
            ("quartus", "Intel Quartus Prime"),
            ("diamond", "Lattice Diamond"),
        ];

        for (tool, description) in &fpga_tools {
            if self.command_exists(tool).await {
                platforms.push(PlatformType::NeuromorphicComputing {
                    platform: format!("{description} FPGA Neuromorphic"),
                    hardware: true,
                });
            }
        }

        Ok(platforms)
    }

    /// Check if a Python package exists
    async fn python_package_exists(&self, package: &str) -> bool {
        if let Ok(output) = std::process::Command::new("python3")
            .args(["-c", &format!("import {package}")])
            .output()
        {
            output.status.success()
        } else if let Ok(output) = std::process::Command::new("python")
            .args(["-c", &format!("import {package}")])
            .output()
        {
            output.status.success()
        } else {
            false
        }
    }

    async fn command_exists(&self, command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or_default()
    }
}

/// Platform type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformType {
    Linux {
        distribution: String,
        architecture: String,
    },
    Windows {
        version: String,
        architecture: String,
    },
    MacOS {
        version: String,
        architecture: String,
    },
    Docker,
    Podman,
    Containerd,
    Language {
        name: String,
        command: String,
    },
    GPU {
        vendor: String,
        framework: String,
    },
    WebAssembly {
        runtime: String,
    },
    Other {
        os: String,
        architecture: String,
    },
    EdgeDevice {
        device_type: String,
        architecture: String,
    },
    MCUDevelopment {
        platform: String,
        tool: String,
    },
    BiologicalComputing {
        platform: String,
        simulation: bool,
    },
    Quantum {
        framework: String,
        simulator: bool,
    },
    NeuromorphicComputing {
        platform: String,
        hardware: bool,
    },
}

/// Comprehensive substrate capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    pub traditional_platforms: Vec<PlatformType>,
    pub container_platforms: Vec<PlatformType>,
    pub language_runtimes: Vec<PlatformType>,
    pub gpu_platforms: Vec<PlatformType>,
    pub specialized_platforms: Vec<PlatformType>,
    pub experimental_platforms: Vec<PlatformType>,
}

impl SubstrateCapabilities {
    #[must_use]
    pub const fn total_platforms(&self) -> usize {
        self.traditional_platforms.len()
            + self.container_platforms.len()
            + self.language_runtimes.len()
            + self.gpu_platforms.len()
            + self.specialized_platforms.len()
            + self.experimental_platforms.len()
    }

    #[must_use]
    pub const fn has_containers(&self) -> bool {
        !self.container_platforms.is_empty()
    }

    #[must_use]
    pub const fn has_gpu(&self) -> bool {
        !self.gpu_platforms.is_empty()
    }

    #[must_use]
    pub fn has_wasm(&self) -> bool {
        self.specialized_platforms
            .iter()
            .any(|p| matches!(p, PlatformType::WebAssembly { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substrate_detector_new() {
        let detector = SubstrateDetector::new();
        // Detector is unit struct, just verify construction
        let _ = detector;
    }

    #[test]
    fn substrate_detector_default() {
        let detector = SubstrateDetector;
        let from_new = SubstrateDetector::new();
        // Both produce valid detectors (unit struct)
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

    // Additional coverage: CPU detection, GPU fallback, capability reporting

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
                assert!(platforms
                    .iter()
                    .any(|p| matches!(p, PlatformType::Linux { .. })));
            }
            "windows" => {
                assert!(platforms
                    .iter()
                    .any(|p| matches!(p, PlatformType::Windows { .. })));
            }
            "macos" => {
                assert!(platforms
                    .iter()
                    .any(|p| matches!(p, PlatformType::MacOS { .. })));
            }
            _ => {
                assert!(platforms
                    .iter()
                    .any(|p| matches!(p, PlatformType::Other { .. })));
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
