// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform capability detection
//!
//! This module provides the detection logic for identifying available computing platforms,
//! runtimes, and system capabilities across traditional, biological, quantum, and
//! experimental computing paradigms.

mod gpu;
mod helpers;

use super::types::*;
use toadstool::ToadStoolResult;

impl UniversalSubstrateCapabilities {
    /// Detect all available substrate capabilities
    ///
    /// Performs comprehensive detection across all platform categories.
    pub async fn detect_all() -> ToadStoolResult<Self> {
        let traditional_platforms = Self::detect_traditional_platforms().await?;
        let biological_platforms = Self::detect_biological_platforms().await?;
        let neuromorphic_platforms = Self::detect_neuromorphic_platforms().await?;
        let quantum_platforms = Self::detect_quantum_platforms().await?;
        let edge_iot_platforms = Self::detect_edge_iot_platforms().await?;
        let container_platforms = Self::detect_container_platforms().await?;
        let language_runtimes = Self::detect_language_runtimes().await?;
        let operating_systems = Self::detect_operating_systems().await?;
        let specialized_architectures = Self::detect_specialized_architectures().await?;
        let experimental_platforms = Self::detect_experimental_platforms().await?;

        Ok(Self {
            traditional_platforms,
            biological_platforms,
            neuromorphic_platforms,
            quantum_platforms,
            edge_iot_platforms,
            container_platforms,
            language_runtimes,
            operating_systems,
            specialized_architectures,
            experimental_platforms,
        })
    }

    /// Detect traditional computing platforms (x86, ARM, RISC-V, etc.)
    async fn detect_traditional_platforms() -> ToadStoolResult<Vec<TraditionalPlatform>> {
        let mut platforms = Vec::new();

        let cpu_info = helpers::get_cpu_info();
        let memory_gb = helpers::get_memory_gb();

        match std::env::consts::ARCH {
            "x86_64" => {
                platforms.push(TraditionalPlatform::X86_64 {
                    cpu_model: cpu_info.model,
                    cores: cpu_info.cores,
                    threads: cpu_info.threads,
                    cache_mb: cpu_info.cache_mb,
                    memory_gb,
                    features: cpu_info.features,
                });
            }
            "aarch64" => {
                platforms.push(TraditionalPlatform::ARM64 {
                    cpu_model: cpu_info.model,
                    cores: cpu_info.cores,
                    big_little: cpu_info.big_little,
                    memory_gb,
                    features: cpu_info.features,
                });
            }
            _ => {}
        }

        Ok(platforms)
    }

    /// Detect biological computing platforms
    async fn detect_biological_platforms() -> ToadStoolResult<Vec<BiologicalComputingPlatform>> {
        Ok(vec![])
    }

    /// Detect neuromorphic computing platforms
    async fn detect_neuromorphic_platforms() -> ToadStoolResult<Vec<NeuromorphicPlatform>> {
        let mut platforms = Vec::new();

        #[cfg(target_os = "linux")]
        {
            let dev_akida = std::path::Path::new("/dev");
            if dev_akida.is_dir()
                && let Ok(entries) = std::fs::read_dir(dev_akida)
            {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("akida") {
                        platforms.push(NeuromorphicPlatform::NeuromorphicChip {
                            chip_name: "Akida".to_string(),
                            manufacturer: "BrainChip".to_string(),
                            core_count: 1,
                            neuron_count_per_core: 1_000_000,
                            synapse_count_per_core: 4_000_000,
                            power_consumption_mw: 500.0,
                        });
                        break;
                    }
                }
            }

            let sys_akida = std::path::Path::new("/sys/class/akida");
            if platforms.is_empty() && sys_akida.is_dir() {
                platforms.push(NeuromorphicPlatform::NeuromorphicChip {
                    chip_name: "Akida".to_string(),
                    manufacturer: "BrainChip".to_string(),
                    core_count: 1,
                    neuron_count_per_core: 1_000_000,
                    synapse_count_per_core: 4_000_000,
                    power_consumption_mw: 500.0,
                });
            }
        }

        Ok(platforms)
    }

    /// Detect quantum computing platforms
    async fn detect_quantum_platforms() -> ToadStoolResult<Vec<QuantumPlatform>> {
        Ok(vec![])
    }

    /// Detect edge/IoT platforms
    async fn detect_edge_iot_platforms() -> ToadStoolResult<Vec<EdgeIoTPlatform>> {
        let mut platforms = Vec::new();

        #[cfg(target_os = "linux")]
        {
            let has_gpio = std::path::Path::new("/sys/class/gpio").exists();

            let mut serial_count = 0u32;
            let dev = std::path::Path::new("/dev");
            if dev.is_dir()
                && let Ok(entries) = std::fs::read_dir(dev)
            {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let name = file_name.to_string_lossy();
                    if name.starts_with("ttyUSB") || name.starts_with("ttyACM") {
                        serial_count += 1;
                    }
                }
            }

            if has_gpio && serial_count > 0 {
                platforms.push(EdgeIoTPlatform::SingleBoardComputer {
                    board: "Generic SBC".to_string(),
                    soc: "unknown".to_string(),
                    ram_mb: 512,
                    storage_type: "unknown".to_string(),
                    connectivity: vec!["GPIO".to_string(), "Serial".to_string()],
                });
            } else if has_gpio {
                platforms.push(EdgeIoTPlatform::SingleBoardComputer {
                    board: "Generic SBC".to_string(),
                    soc: "unknown".to_string(),
                    ram_mb: 512,
                    storage_type: "unknown".to_string(),
                    connectivity: vec!["GPIO".to_string()],
                });
            } else if serial_count > 0 {
                platforms.push(EdgeIoTPlatform::Microcontroller {
                    chip: "USB Serial".to_string(),
                    architecture: "unknown".to_string(),
                    flash_kb: 0,
                    ram_kb: 0,
                    clock_speed_mhz: 0,
                    gpio_pins: 0,
                });
            }
        }

        Ok(platforms)
    }

    /// Detect container platforms
    async fn detect_container_platforms() -> ToadStoolResult<Vec<ContainerPlatform>> {
        let mut platforms = Vec::new();

        if helpers::check_command_exists("docker") {
            platforms.push(ContainerPlatform::Docker {
                version: helpers::get_command_version("docker --version"),
                features: vec!["buildx".to_string(), "compose".to_string()],
            });
        }

        if helpers::check_command_exists("podman") {
            platforms.push(ContainerPlatform::Podman {
                version: helpers::get_command_version("podman --version"),
                rootless: true,
            });
        }

        if helpers::check_command_exists("kubectl") {
            platforms.push(ContainerPlatform::Kubernetes {
                version: helpers::get_command_version("kubectl version --client --short"),
                distribution: "vanilla".to_string(),
            });
        }

        if helpers::check_command_exists("wasmtime") {
            platforms.push(ContainerPlatform::Wasmtime {
                version: helpers::get_command_version("wasmtime --version"),
                features: vec!["async".to_string()],
            });
        }

        if helpers::check_command_exists("wasmer") {
            platforms.push(ContainerPlatform::Wasmer {
                version: helpers::get_command_version("wasmer --version"),
                backends: vec!["singlepass".to_string(), "cranelift".to_string()],
            });
        }

        Ok(platforms)
    }

    /// Detect language runtimes
    async fn detect_language_runtimes() -> ToadStoolResult<Vec<LanguageRuntime>> {
        let mut runtimes = Vec::new();

        if helpers::check_command_exists("rustc") {
            runtimes.push(LanguageRuntime::Rust {
                version: helpers::get_command_version("rustc --version"),
                target_triple: helpers::get_rust_target_triple(),
                features: vec!["std".to_string()],
            });
        }

        if helpers::check_command_exists("gcc") {
            runtimes.push(LanguageRuntime::C {
                compiler: "gcc".to_string(),
                standard: "c17".to_string(),
                optimizations: vec!["O2".to_string()],
            });
        }

        if helpers::check_command_exists("go") {
            runtimes.push(LanguageRuntime::Go {
                version: helpers::get_command_version("go version"),
                goos: std::env::consts::OS.to_string(),
                goarch: std::env::consts::ARCH.to_string(),
            });
        }

        if helpers::check_command_exists("python3") {
            runtimes.push(LanguageRuntime::Python {
                version: helpers::get_command_version("python3 --version"),
                implementation: "CPython".to_string(),
                features: vec!["asyncio".to_string()],
            });
        }

        if helpers::check_command_exists("node") {
            runtimes.push(LanguageRuntime::JavaScript {
                engine: "V8".to_string(),
                version: helpers::get_command_version("node --version"),
                features: vec!["async".to_string()],
            });
        }

        if helpers::check_command_exists("java") {
            runtimes.push(LanguageRuntime::Java {
                version: helpers::get_command_version("java --version"),
                vm: "OpenJDK".to_string(),
                gc: "G1".to_string(),
            });
        }

        if helpers::check_command_exists("bash") {
            runtimes.push(LanguageRuntime::Bash {
                version: helpers::get_command_version("bash --version"),
                features: vec!["pipefail".to_string()],
            });
        }

        Ok(runtimes)
    }

    /// Detect operating systems
    async fn detect_operating_systems() -> ToadStoolResult<Vec<OperatingSystemSupport>> {
        let mut systems = Vec::new();

        match std::env::consts::OS {
            "linux" => {
                systems.push(OperatingSystemSupport::Linux {
                    distribution: helpers::get_linux_distribution(),
                    kernel_version: helpers::get_kernel_version(),
                    init_system: helpers::get_init_system(),
                    package_manager: helpers::get_package_manager(),
                });
            }
            "macos" => {
                systems.push(OperatingSystemSupport::MacOS {
                    version: helpers::get_macos_version(),
                    architecture: std::env::consts::ARCH.to_string(),
                    frameworks: helpers::get_macos_frameworks(),
                });
            }
            "windows" => {
                systems.push(OperatingSystemSupport::Windows {
                    version: helpers::get_windows_version(),
                    edition: "Pro".to_string(),
                    features: helpers::get_windows_features(),
                    subsystems: helpers::get_windows_subsystems(),
                });
            }
            _ => {}
        }

        Ok(systems)
    }

    /// Detect specialized architectures (GPUs, TPUs, etc.)
    async fn detect_specialized_architectures() -> ToadStoolResult<Vec<SpecializedArchitecture>> {
        let mut architectures = Vec::new();

        if helpers::check_command_exists("nvcc") {
            architectures.push(SpecializedArchitecture::CUDA {
                version: gpu::get_cuda_version(),
                compute_capability: gpu::get_cuda_compute_capability(),
                memory_gb: gpu::get_gpu_memory_gb(),
            });
        }

        if helpers::check_command_exists("rocm-smi") {
            architectures.push(SpecializedArchitecture::ROCm {
                version: gpu::get_rocm_version(),
                gfx_version: gpu::get_rocm_gfx_version(),
                memory_gb: gpu::get_gpu_memory_gb(),
            });
        }

        if gpu::check_opencl_support() {
            architectures.push(SpecializedArchitecture::OpenCL {
                version: gpu::get_opencl_version(),
                device_type: gpu::get_opencl_device_type(),
                compute_units: gpu::get_opencl_compute_units(),
            });
        }

        Ok(architectures)
    }

    /// Detect experimental platforms
    async fn detect_experimental_platforms() -> ToadStoolResult<Vec<ExperimentalPlatform>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_all() {
        let caps = UniversalSubstrateCapabilities::detect_all().await;
        assert!(caps.is_ok());

        let caps = caps.unwrap();
        assert!(!caps.traditional_platforms.is_empty());
        assert!(!caps.operating_systems.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_traditional() {
        let platforms = UniversalSubstrateCapabilities::detect_traditional_platforms().await;
        assert!(platforms.is_ok());
        assert!(!platforms.unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_container() {
        let platforms = UniversalSubstrateCapabilities::detect_container_platforms().await;
        assert!(platforms.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_language() {
        let runtimes = UniversalSubstrateCapabilities::detect_language_runtimes().await;
        assert!(runtimes.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_biological_platforms() {
        let platforms = UniversalSubstrateCapabilities::detect_biological_platforms().await;
        assert!(platforms.is_ok());
        assert!(platforms.unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_neuromorphic_platforms() {
        let platforms = UniversalSubstrateCapabilities::detect_neuromorphic_platforms().await;
        assert!(platforms.is_ok());
        assert!(platforms.unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_quantum_platforms() {
        let platforms = UniversalSubstrateCapabilities::detect_quantum_platforms().await;
        assert!(platforms.is_ok());
        assert!(platforms.unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_edge_iot_platforms() {
        let platforms = UniversalSubstrateCapabilities::detect_edge_iot_platforms().await;
        assert!(platforms.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_experimental_platforms() {
        let platforms = UniversalSubstrateCapabilities::detect_experimental_platforms().await;
        assert!(platforms.is_ok());
        assert!(platforms.unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_operating_systems() {
        let systems = UniversalSubstrateCapabilities::detect_operating_systems().await;
        assert!(systems.is_ok());
        assert!(!systems.unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_specialized_architectures() {
        let archs = UniversalSubstrateCapabilities::detect_specialized_architectures().await;
        assert!(archs.is_ok());
    }
}
