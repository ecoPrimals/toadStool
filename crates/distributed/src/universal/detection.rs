// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform capability detection
//!
//! This module provides the detection logic for identifying available computing platforms,
//! runtimes, and system capabilities across traditional, biological, quantum, and
//! experimental computing paradigms.

use super::types::*;
use toadstool::ToadStoolResult;

/// CPU information structure
#[derive(Debug, Clone)]
struct CpuInfo {
    model: String,
    cores: u32,
    threads: u32,
    cache_mb: u32,
    big_little: bool,
    features: Vec<String>,
}

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

        // Detect CPU architecture and capabilities
        let cpu_info = Self::get_cpu_info();
        let memory_gb = Self::get_memory_gb();

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
        // Biological platforms are mostly aspirational at this point
        // Real detection would involve specialized lab equipment
        Ok(vec![])
    }

    /// Detect neuromorphic computing platforms
    async fn detect_neuromorphic_platforms() -> ToadStoolResult<Vec<NeuromorphicPlatform>> {
        let mut platforms = Vec::new();

        #[cfg(target_os = "linux")]
        {
            // Scan for Akida NPU devices
            let dev_akida = std::path::Path::new("/dev");
            if dev_akida.is_dir() {
                if let Ok(entries) = std::fs::read_dir(dev_akida) {
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
        // Quantum platforms require specialized detection
        // Would check for cloud quantum service credentials, local quantum simulators
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
            if dev.is_dir() {
                if let Ok(entries) = std::fs::read_dir(dev) {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name();
                        let name = file_name.to_string_lossy();
                        if name.starts_with("ttyUSB") || name.starts_with("ttyACM") {
                            serial_count += 1;
                        }
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

        // Check for Docker
        if Self::check_command_exists("docker") {
            platforms.push(ContainerPlatform::Docker {
                version: Self::get_command_version("docker --version"),
                features: vec!["buildx".to_string(), "compose".to_string()],
            });
        }

        // Check for Podman
        if Self::check_command_exists("podman") {
            platforms.push(ContainerPlatform::Podman {
                version: Self::get_command_version("podman --version"),
                rootless: true,
            });
        }

        // Check for Kubernetes
        if Self::check_command_exists("kubectl") {
            platforms.push(ContainerPlatform::Kubernetes {
                version: Self::get_command_version("kubectl version --client --short"),
                distribution: "vanilla".to_string(),
            });
        }

        // Check for Wasmtime
        if Self::check_command_exists("wasmtime") {
            platforms.push(ContainerPlatform::Wasmtime {
                version: Self::get_command_version("wasmtime --version"),
                features: vec!["async".to_string()],
            });
        }

        // Check for Wasmer
        if Self::check_command_exists("wasmer") {
            platforms.push(ContainerPlatform::Wasmer {
                version: Self::get_command_version("wasmer --version"),
                backends: vec!["singlepass".to_string(), "cranelift".to_string()],
            });
        }

        Ok(platforms)
    }

    /// Detect language runtimes
    async fn detect_language_runtimes() -> ToadStoolResult<Vec<LanguageRuntime>> {
        let mut runtimes = Vec::new();

        // Systems languages
        if Self::check_command_exists("rustc") {
            runtimes.push(LanguageRuntime::Rust {
                version: Self::get_command_version("rustc --version"),
                target_triple: Self::get_rust_target_triple(),
                features: vec!["std".to_string()],
            });
        }

        if Self::check_command_exists("gcc") {
            runtimes.push(LanguageRuntime::C {
                compiler: "gcc".to_string(),
                standard: "c17".to_string(),
                optimizations: vec!["O2".to_string()],
            });
        }

        if Self::check_command_exists("go") {
            runtimes.push(LanguageRuntime::Go {
                version: Self::get_command_version("go version"),
                goos: std::env::consts::OS.to_string(),
                goarch: std::env::consts::ARCH.to_string(),
            });
        }

        // Memory-managed languages
        if Self::check_command_exists("python3") {
            runtimes.push(LanguageRuntime::Python {
                version: Self::get_command_version("python3 --version"),
                implementation: "CPython".to_string(),
                features: vec!["asyncio".to_string()],
            });
        }

        if Self::check_command_exists("node") {
            runtimes.push(LanguageRuntime::JavaScript {
                engine: "V8".to_string(),
                version: Self::get_command_version("node --version"),
                features: vec!["async".to_string()],
            });
        }

        if Self::check_command_exists("java") {
            runtimes.push(LanguageRuntime::Java {
                version: Self::get_command_version("java --version"),
                vm: "OpenJDK".to_string(),
                gc: "G1".to_string(),
            });
        }

        // Scripting languages
        if Self::check_command_exists("bash") {
            runtimes.push(LanguageRuntime::Bash {
                version: Self::get_command_version("bash --version"),
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
                    distribution: Self::get_linux_distribution(),
                    kernel_version: Self::get_kernel_version(),
                    init_system: Self::get_init_system(),
                    package_manager: Self::get_package_manager(),
                });
            }
            "macos" => {
                systems.push(OperatingSystemSupport::MacOS {
                    version: Self::get_macos_version(),
                    architecture: std::env::consts::ARCH.to_string(),
                    frameworks: Self::get_macos_frameworks(),
                });
            }
            "windows" => {
                systems.push(OperatingSystemSupport::Windows {
                    version: Self::get_windows_version(),
                    edition: "Pro".to_string(),
                    features: Self::get_windows_features(),
                    subsystems: Self::get_windows_subsystems(),
                });
            }
            _ => {}
        }

        Ok(systems)
    }

    /// Detect specialized architectures (GPUs, TPUs, etc.)
    async fn detect_specialized_architectures() -> ToadStoolResult<Vec<SpecializedArchitecture>> {
        let mut architectures = Vec::new();

        // Check for CUDA
        if Self::check_command_exists("nvcc") {
            architectures.push(SpecializedArchitecture::CUDA {
                version: Self::get_cuda_version(),
                compute_capability: Self::get_cuda_compute_capability(),
                memory_gb: Self::get_gpu_memory_gb(),
            });
        }

        // Check for ROCm
        if Self::check_command_exists("rocm-smi") {
            architectures.push(SpecializedArchitecture::ROCm {
                version: Self::get_rocm_version(),
                gfx_version: Self::get_rocm_gfx_version(),
                memory_gb: Self::get_gpu_memory_gb(),
            });
        }

        // Check for OpenCL
        if Self::check_opencl_support() {
            architectures.push(SpecializedArchitecture::OpenCL {
                version: Self::get_opencl_version(),
                device_type: Self::get_opencl_device_type(),
                compute_units: Self::get_opencl_compute_units(),
            });
        }

        Ok(architectures)
    }

    /// Detect experimental platforms
    async fn detect_experimental_platforms() -> ToadStoolResult<Vec<ExperimentalPlatform>> {
        // Experimental platforms are mostly aspirational
        // Real detection would involve specialized lab equipment
        Ok(vec![])
    }

    // === Helper Methods ===

    /// Get CPU information
    fn get_cpu_info() -> CpuInfo {
        let cores = std::thread::available_parallelism()
            .map(|p| u32::try_from(p.get()).unwrap_or(4))
            .unwrap_or(4);

        #[cfg(target_os = "linux")]
        {
            if let Ok(info) = Self::parse_cpuinfo_linux() {
                return info;
            }
        }

        // Fallback for non-Linux or parse failure
        CpuInfo {
            model: "Generic CPU".to_string(),
            cores,
            threads: cores,
            cache_mb: 8,
            big_little: false,
            features: vec!["sse4.2".to_string(), "avx2".to_string()],
        }
    }

    #[cfg(target_os = "linux")]
    fn parse_cpuinfo_linux() -> Result<CpuInfo, ()> {
        let content = std::fs::read_to_string("/proc/cpuinfo").map_err(|_| ())?;

        let cores = std::thread::available_parallelism()
            .map(|p| u32::try_from(p.get()).unwrap_or(4))
            .unwrap_or(4);

        let mut model = "Generic CPU".to_string();
        let mut cache_mb = 8u32;
        let mut flags = Vec::new();
        let mut cpu_parts: std::collections::HashSet<String> = std::collections::HashSet::new();

        for block in content.split("\n\n") {
            for line in block.lines() {
                if let Some((key, val)) = line.split_once(':') {
                    let key = key.trim();
                    let val = val.trim();
                    match key {
                        "model name" | "Model" => model = val.to_string(),
                        "cache size" => {
                            if let Some(kb_str) = val.split_whitespace().next() {
                                if let Ok(kb) = kb_str.parse::<u32>() {
                                    cache_mb = kb.div_ceil(1024);
                                }
                            }
                        }
                        "flags" | "Features" => {
                            flags = val
                                .split_whitespace()
                                .filter(|s| s.len() > 2)
                                .map(String::from)
                                .collect();
                        }
                        "CPU part" => {
                            cpu_parts.insert(val.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        let features = if flags.is_empty() {
            vec!["sse4.2".to_string(), "avx2".to_string()]
        } else {
            flags
        };

        let big_little = cpu_parts.len() > 1;

        Ok(CpuInfo {
            model,
            cores,
            threads: cores,
            cache_mb,
            big_little,
            features,
        })
    }

    /// Get system memory in gigabytes
    fn get_memory_gb() -> u32 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                let gb = kb.div_ceil(1024 * 1024);
                                return gb.min(u32::MAX as u64) as u32;
                            }
                        }
                        break;
                    }
                }
            }
        }

        8
    }

    /// Check if a command exists in PATH
    fn check_command_exists(command: &str) -> bool {
        std::process::Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or_default()
    }

    /// Get version string from a command
    fn get_command_version(command: &str) -> String {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_or_else(
                |_| "unknown".to_string(),
                |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )
    }

    /// Get Rust target triple
    fn get_rust_target_triple() -> String {
        std::process::Command::new("rustc")
            .arg("--print")
            .arg("target-triple")
            .output()
            .map_or_else(
                |_| "unknown".to_string(),
                |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )
    }

    /// Get Linux distribution name
    fn get_linux_distribution() -> String {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        let val = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                        if !val.is_empty() {
                            return val.to_string();
                        }
                        break;
                    }
                }
                for line in content.lines() {
                    if line.starts_with("NAME=") {
                        let val = line.trim_start_matches("NAME=").trim_matches('"');
                        if !val.is_empty() {
                            return val.to_string();
                        }
                        break;
                    }
                }
            }
        }
        "unknown".to_string()
    }

    /// Get kernel version
    fn get_kernel_version() -> String {
        std::process::Command::new("uname")
            .arg("-r")
            .output()
            .map_or_else(
                |_| "unknown".to_string(),
                |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )
    }

    /// Get init system type
    fn get_init_system() -> String {
        if std::path::Path::new("/run/systemd/system").exists() {
            "systemd".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Get package manager type
    fn get_package_manager() -> String {
        if Self::check_command_exists("apt") {
            "apt".to_string()
        } else if Self::check_command_exists("yum") {
            "yum".to_string()
        } else if Self::check_command_exists("pacman") {
            "pacman".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Get macOS version
    fn get_macos_version() -> String {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_or_else(
                |_| "unknown".to_string(),
                |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )
    }

    /// Get macOS frameworks
    fn get_macos_frameworks() -> Vec<String> {
        vec!["Foundation".to_string(), "CoreFoundation".to_string()]
    }

    /// Get Windows version
    fn get_windows_version() -> String {
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("cmd")
                .args(["/c", "ver"])
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                let s = s.trim();
                // Extract version number from output like "Microsoft Windows [Version 10.0.19045.3803]"
                if let Some(start) = s.find("Version ") {
                    let rest = &s[start + 8..];
                    if let Some(end) = rest.find(']') {
                        let ver = rest[..end].trim();
                        if !ver.is_empty() {
                            return ver.to_string();
                        }
                    }
                }
            }
        }
        "10".to_string()
    }

    /// Get Windows features
    fn get_windows_features() -> Vec<String> {
        vec!["PowerShell".to_string(), "WSL".to_string()]
    }

    /// Get Windows subsystems
    fn get_windows_subsystems() -> Vec<String> {
        vec!["Win32".to_string(), "WSL".to_string()]
    }

    /// Get CUDA version
    fn get_cuda_version() -> String {
        std::process::Command::new("nvcc")
            .arg("--version")
            .output()
            .map_or_else(
                |_| "unknown".to_string(),
                |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )
    }

    /// Get CUDA compute capability
    fn get_cuda_compute_capability() -> String {
        if Self::check_command_exists("nvidia-smi") {
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
                .output()
            {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout);
                    let cap = s.trim().split('\n').next().unwrap_or("").trim();
                    if !cap.is_empty() {
                        return cap.to_string();
                    }
                }
            }
        }
        "unknown".to_string()
    }

    /// Get GPU memory in gigabytes
    fn get_gpu_memory_gb() -> u32 {
        if Self::check_command_exists("nvidia-smi") {
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout);
                    if let Some(mb_str) = s
                        .trim()
                        .split('\n')
                        .next()
                        .and_then(|l| l.split(',').next())
                    {
                        let mb_str = mb_str.trim();
                        if let Ok(mb) = mb_str.parse::<u32>() {
                            return mb.div_ceil(1024);
                        }
                    }
                }
            }
        }
        0
    }

    /// Get ROCm version
    fn get_rocm_version() -> String {
        if let Ok(ver) = std::fs::read_to_string("/opt/rocm/.info/version") {
            let ver = ver.trim();
            if !ver.is_empty() {
                return ver.to_string();
            }
        }
        if Self::check_command_exists("rocm-smi") {
            if let Ok(output) = std::process::Command::new("rocm-smi")
                .arg("--showversion")
                .output()
            {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout);
                    let first_line = s.lines().next().unwrap_or("").trim();
                    if !first_line.is_empty() {
                        return first_line.to_string();
                    }
                }
            }
        }
        "unknown".to_string()
    }

    /// Get ROCm GFX version
    fn get_rocm_gfx_version() -> String {
        if Self::check_command_exists("rocm-smi") {
            if let Ok(output) = std::process::Command::new("rocm-smi")
                .arg("--showproductname")
                .output()
            {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout);
                    for line in s.lines() {
                        let line = line.trim();
                        if line.contains("gfx") {
                            if let Some(gfx) =
                                line.split_whitespace().find(|w| w.starts_with("gfx"))
                            {
                                return gfx.to_string();
                            }
                        }
                    }
                }
            }
        }
        "unknown".to_string()
    }

    /// Check for OpenCL support
    fn check_opencl_support() -> bool {
        #[cfg(target_os = "linux")]
        {
            let vendors = std::path::Path::new("/etc/OpenCL/vendors");
            if vendors.is_dir() {
                if let Ok(entries) = std::fs::read_dir(vendors) {
                    for entry in entries.flatten() {
                        if entry.path().extension().is_some_and(|e| e == "icd") {
                            return true;
                        }
                    }
                }
            }
        }
        Self::check_command_exists("clinfo")
    }

    /// Get OpenCL version
    fn get_opencl_version() -> String {
        "2.0".to_string()
    }

    /// Get OpenCL device type
    fn get_opencl_device_type() -> String {
        "GPU".to_string()
    }

    /// Get OpenCL compute units
    const fn get_opencl_compute_units() -> u32 {
        64
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
        // At minimum should detect traditional platform and OS
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

    #[test]
    fn test_cpu_info() {
        let info = UniversalSubstrateCapabilities::get_cpu_info();
        assert!(info.cores > 0);
    }

    #[test]
    fn test_command_detection() {
        // Test with a command that should always exist on Unix systems
        let exists = UniversalSubstrateCapabilities::check_command_exists("sh");
        assert!(exists);

        // Test with a command that likely doesn't exist
        let not_exists =
            UniversalSubstrateCapabilities::check_command_exists("nonexistent_command_xyz");
        assert!(!not_exists);
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
        // May be empty (no GPIO/serial) or populated (edge devices detected on Linux)
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

    #[test]
    fn test_get_memory_gb() {
        let gb = UniversalSubstrateCapabilities::get_memory_gb();
        assert!(gb > 0);
    }

    #[test]
    fn test_get_command_version() {
        let version = UniversalSubstrateCapabilities::get_command_version("echo test");
        assert!(!version.is_empty() || version == "unknown");
    }

    #[test]
    fn test_get_linux_distribution() {
        let dist = UniversalSubstrateCapabilities::get_linux_distribution();
        assert!(!dist.is_empty());
    }

    #[test]
    fn test_get_init_system() {
        let init = UniversalSubstrateCapabilities::get_init_system();
        assert!(!init.is_empty());
    }

    #[test]
    fn test_get_package_manager() {
        let pkg = UniversalSubstrateCapabilities::get_package_manager();
        assert!(!pkg.is_empty());
    }

    #[test]
    fn test_get_macos_frameworks() {
        let fw = UniversalSubstrateCapabilities::get_macos_frameworks();
        assert!(!fw.is_empty());
    }

    #[test]
    fn test_get_windows_version() {
        let ver = UniversalSubstrateCapabilities::get_windows_version();
        assert!(!ver.is_empty());
    }

    #[test]
    fn test_check_opencl_support() {
        let support = UniversalSubstrateCapabilities::check_opencl_support();
        // Result is environment-dependent; verify it's a valid bool
        let _ = support;
    }

    #[test]
    fn test_get_opencl_compute_units() {
        let units = UniversalSubstrateCapabilities::get_opencl_compute_units();
        assert!(units > 0);
    }

    #[test]
    fn test_get_rust_target_triple() {
        let triple = UniversalSubstrateCapabilities::get_rust_target_triple();
        assert!(triple == "unknown" || triple.is_empty() || triple.contains("-"));
    }

    #[test]
    fn test_get_opencl_version() {
        let version = UniversalSubstrateCapabilities::get_opencl_version();
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_get_opencl_device_type() {
        let device_type = UniversalSubstrateCapabilities::get_opencl_device_type();
        assert_eq!(device_type, "GPU");
    }

    #[test]
    fn test_get_windows_features() {
        let features = UniversalSubstrateCapabilities::get_windows_features();
        assert!(!features.is_empty());
        assert!(features.contains(&"PowerShell".to_string()));
    }

    #[test]
    fn test_get_windows_subsystems() {
        let subsystems = UniversalSubstrateCapabilities::get_windows_subsystems();
        assert!(!subsystems.is_empty());
        assert!(subsystems.contains(&"Win32".to_string()));
    }

    #[test]
    fn test_get_cuda_version() {
        let version = UniversalSubstrateCapabilities::get_cuda_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_get_cuda_compute_capability() {
        let cap = UniversalSubstrateCapabilities::get_cuda_compute_capability();
        assert!(!cap.is_empty());
    }

    #[test]
    fn test_get_gpu_memory_gb() {
        let gb = UniversalSubstrateCapabilities::get_gpu_memory_gb();
        let _ = gb;
    }

    #[test]
    fn test_get_rocm_version() {
        let version = UniversalSubstrateCapabilities::get_rocm_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_get_rocm_gfx_version() {
        let gfx = UniversalSubstrateCapabilities::get_rocm_gfx_version();
        assert!(!gfx.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_cpuinfo_linux_fallback() {
        let result = UniversalSubstrateCapabilities::parse_cpuinfo_linux();
        if let Ok(info) = result {
            assert!(info.cores > 0);
            assert!(!info.model.is_empty());
        }
    }
}
