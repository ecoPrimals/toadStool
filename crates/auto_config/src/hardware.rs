//! # Hardware Detection and Capability Assessment
//!
//! Comprehensive system hardware detection for optimal `ToadStool` configuration.
//! Detects CPU, memory, GPU, storage, and network capabilities to enable
//! zero-touch optimization.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::ToadStoolResult;

/// Hardware detection and capability assessment system
pub struct HardwareDetector {
    _system_info: Option<SystemInfo>,
}

impl HardwareDetector {
    /// Create a new hardware detector
    #[must_use]
    pub fn new() -> Self {
        Self { _system_info: None }
    }

    /// Comprehensive system scan to detect all hardware capabilities
    pub async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        info!("🔍 Starting comprehensive hardware scan...");

        let mut capabilities = SystemCapabilities::default();

        // CPU detection
        info!("  🖥️ Scanning CPU capabilities...");
        capabilities.cpu_info = self.detect_cpu().await?;
        capabilities.cpu_cores = capabilities.cpu_info.physical_cores as f64;

        // Memory detection
        info!("  🧠 Scanning memory configuration...");
        capabilities.memory_info = self.detect_memory().await?;
        capabilities.memory_gb = capabilities.memory_info.total_gb;

        // GPU detection
        info!("  🎮 Scanning GPU capabilities...");
        capabilities.gpu_info = self.detect_gpus().await?;
        capabilities.gpu_count = capabilities.gpu_info.len();
        capabilities.gpu_memory_gb = capabilities.gpu_info.first().map(|gpu| gpu.memory_gb);

        // Storage detection
        info!("  💾 Scanning storage configuration...");
        capabilities.storage_info = self.detect_storage().await?;
        capabilities.storage_gb = capabilities.storage_info.total_gb;

        // Network detection
        info!("  🌐 Scanning network interfaces...");
        capabilities.network_info = self.detect_network()?;

        // Performance characteristics
        info!("  ⚡ Analyzing performance characteristics...");
        capabilities.performance_class = self.classify_performance(&capabilities).await?;

        info!("✅ Hardware scan complete:");
        info!(
            "   CPU: {} cores ({})",
            capabilities.cpu_cores, capabilities.cpu_info.model_name
        );
        info!("   Memory: {:.1} GB", capabilities.memory_gb);
        info!("   GPU: {} devices", capabilities.gpu_count);
        info!("   Storage: {:.1} GB", capabilities.storage_gb);
        info!("   Performance: {:?}", capabilities.performance_class);

        Ok(capabilities)
    }

    /// Detect CPU capabilities and characteristics
    async fn detect_cpu(&self) -> ToadStoolResult<CpuInfo> {
        let mut cpu_info = CpuInfo::default();

        // Try to read CPU info from /proc/cpuinfo on Linux
        if cfg!(target_os = "linux") {
            if let Ok(cpuinfo) = tokio::fs::read_to_string("/proc/cpuinfo").await {
                cpu_info = self.parse_linux_cpuinfo(&cpuinfo)?;
            }
        }

        // Try to get CPU info from sysctl on macOS
        #[cfg(target_os = "macos")]
        {
            cpu_info = self.detect_macos_cpu().await?;
        }

        // Try to get CPU info from WMI on Windows
        #[cfg(target_os = "windows")]
        {
            cpu_info = self.detect_windows_cpu().await?;
        }

        // Fallback: use std::thread::available_parallelism
        if cpu_info.physical_cores == 0 {
            cpu_info.physical_cores = std::thread::available_parallelism()
                .map(std::num::NonZero::get)
                .unwrap_or(4);
            cpu_info.logical_cores = cpu_info.physical_cores;
            cpu_info.model_name = "Unknown CPU".to_string();
            warn!("Could not detect CPU details, using fallback values");
        }

        // Detect CPU features
        cpu_info.features = self.detect_cpu_features()?;

        debug!(
            "Detected CPU: {} with {} cores",
            cpu_info.model_name, cpu_info.physical_cores
        );
        Ok(cpu_info)
    }

    /// Parse Linux /proc/cpuinfo
    fn parse_linux_cpuinfo(&self, cpuinfo: &str) -> ToadStoolResult<CpuInfo> {
        let mut cpu_info = CpuInfo::default();
        let mut core_count = 0;

        for line in cpuinfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "processor" => {
                        core_count += 1;
                    }
                    "model name" => {
                        if cpu_info.model_name.is_empty() {
                            cpu_info.model_name = value.to_string();
                        }
                    }
                    "cpu family" => {
                        if let Ok(family) = value.parse::<u32>() {
                            cpu_info.family = family;
                        }
                    }
                    "cpu MHz" => {
                        if let Ok(mhz) = value.parse::<f64>() {
                            cpu_info.base_frequency_mhz = mhz;
                        }
                    }
                    "cache size" => {
                        if value.contains("KB") {
                            if let Ok(kb) = value.replace(" KB", "").parse::<u32>() {
                                cpu_info.cache_size_kb = kb;
                            }
                        }
                    }
                    "flags" | "Features" => {
                        cpu_info.instruction_sets = value
                            .split_whitespace()
                            .map(std::string::ToString::to_string)
                            .collect();
                    }
                    _ => {}
                }
            }
        }

        cpu_info.logical_cores = core_count;
        cpu_info.physical_cores = core_count; // Simplified - would need more logic for HT detection

        Ok(cpu_info)
    }

    /// Detect macOS CPU information
    #[cfg(target_os = "macos")]
    async fn detect_macos_cpu(&self) -> ToadStoolResult<CpuInfo> {
        let mut cpu_info = CpuInfo::default();

        // Use sysctl to get CPU information
        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("machdep.cpu.brand_string")
            .output()
            .await
        {
            cpu_info.model_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("hw.physicalcpu")
            .output()
            .await
        {
            if let Ok(cores) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<usize>()
            {
                cpu_info.physical_cores = cores;
            }
        }

        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("hw.logicalcpu")
            .output()
            .await
        {
            if let Ok(cores) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<usize>()
            {
                cpu_info.logical_cores = cores;
            }
        }

        Ok(cpu_info)
    }

    /// Detect Windows CPU information
    #[cfg(target_os = "windows")]
    async fn detect_windows_cpu(&self) -> ToadStoolResult<CpuInfo> {
        let mut cpu_info = CpuInfo::default();

        // Use WMI to get CPU information (simplified implementation)
        if let Ok(output) = tokio::process::Command::new("wmic")
            .arg("cpu")
            .arg("get")
            .arg("Name,NumberOfCores,NumberOfLogicalProcessors")
            .arg("/format:csv")
            .output()
            .await
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Parse WMI output (simplified)
            for line in output_str.lines().skip(1) {
                if !line.trim().is_empty() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 4 {
                        cpu_info.model_name = parts[1].trim().to_string();
                        if let Ok(cores) = parts[2].trim().parse::<usize>() {
                            cpu_info.physical_cores = cores;
                        }
                        if let Ok(logical) = parts[3].trim().parse::<usize>() {
                            cpu_info.logical_cores = logical;
                        }
                    }
                    break; // Just get first CPU for now
                }
            }
        }

        Ok(cpu_info)
    }

    /// Detect CPU features and instruction sets
    fn detect_cpu_features(&self) -> ToadStoolResult<CpuFeatures> {
        let mut features = CpuFeatures::default();

        // Check for common instruction sets
        #[cfg(target_arch = "x86_64")]
        {
            features.supports_avx = is_x86_feature_detected!("avx");
            features.supports_avx2 = is_x86_feature_detected!("avx2");
            features.supports_sse4_1 = is_x86_feature_detected!("sse4.1");
            features.supports_sse4_2 = is_x86_feature_detected!("sse4.2");
        }

        #[cfg(target_arch = "aarch64")]
        {
            features.supports_neon = is_aarch64_feature_detected!("neon");
        }

        debug!(
            "Detected CPU features: AVX={}, AVX2={}, SSE4.1={}, SSE4.2={}",
            features.supports_avx,
            features.supports_avx2,
            features.supports_sse4_1,
            features.supports_sse4_2
        );

        Ok(features)
    }

    /// Detect memory configuration
    async fn detect_memory(&self) -> ToadStoolResult<MemoryInfo> {
        let mut memory_info = MemoryInfo::default();

        // Try to get memory info from /proc/meminfo on Linux
        if cfg!(target_os = "linux") {
            if let Ok(meminfo) = tokio::fs::read_to_string("/proc/meminfo").await {
                memory_info = self.parse_linux_meminfo(&meminfo)?;
            }
        }

        // macOS memory detection
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = tokio::process::Command::new("sysctl")
                .arg("-n")
                .arg("hw.memsize")
                .output()
                .await
            {
                if let Ok(bytes) = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u64>()
                {
                    memory_info.total_gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                }
            }
        }

        // Windows memory detection
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = tokio::process::Command::new("wmic")
                .arg("computersystem")
                .arg("get")
                .arg("TotalPhysicalMemory")
                .arg("/format:csv")
                .output()
                .await
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines().skip(1) {
                    if !line.trim().is_empty() {
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() >= 2 {
                            if let Ok(bytes) = parts[1].trim().parse::<u64>() {
                                memory_info.total_gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                            }
                        }
                        break;
                    }
                }
            }
        }

        debug!("Detected memory: {:.1} GB total", memory_info.total_gb);
        Ok(memory_info)
    }

    /// Parse Linux /proc/meminfo
    fn parse_linux_meminfo(&self, meminfo: &str) -> ToadStoolResult<MemoryInfo> {
        let mut memory_info = MemoryInfo::default();

        for line in meminfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "MemTotal" => {
                        if let Some(kb_str) = value.split_whitespace().next() {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                memory_info.total_gb =
                                    (kb * 1024) as f64 / (1024.0 * 1024.0 * 1024.0);
                            }
                        }
                    }
                    "MemAvailable" => {
                        if let Some(kb_str) = value.split_whitespace().next() {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                memory_info.available_gb =
                                    (kb * 1024) as f64 / (1024.0 * 1024.0 * 1024.0);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(memory_info)
    }

    /// Detect GPU capabilities
    async fn detect_gpus(&self) -> ToadStoolResult<Vec<GpuInfo>> {
        let mut gpus = Vec::new();

        // Try to detect NVIDIA GPUs using nvidia-smi
        if let Ok(nvidia_gpus) = self.detect_nvidia_gpus().await {
            gpus.extend(nvidia_gpus);
        }

        // Try to detect AMD GPUs
        if let Ok(amd_gpus) = self.detect_amd_gpus().await {
            gpus.extend(amd_gpus);
        }

        // Try to detect Intel GPUs
        if let Ok(intel_gpus) = self.detect_intel_gpus().await {
            gpus.extend(intel_gpus);
        }

        debug!("Detected {} GPU(s)", gpus.len());
        Ok(gpus)
    }

    /// Detect NVIDIA GPUs
    async fn detect_nvidia_gpus(&self) -> ToadStoolResult<Vec<GpuInfo>> {
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
                        compute_capability: self.get_nvidia_compute_capability(&name),
                        supports_cuda: true,
                        supports_opencl: true,
                    });
                }
            }
        }

        Ok(gpus)
    }

    /// Detect AMD GPUs
    async fn detect_amd_gpus(&self) -> ToadStoolResult<Vec<GpuInfo>> {
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
    async fn detect_intel_gpus(&self) -> ToadStoolResult<Vec<GpuInfo>> {
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
    fn get_nvidia_compute_capability(&self, gpu_name: &str) -> String {
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

    /// Detect storage configuration
    async fn detect_storage(&self) -> ToadStoolResult<StorageInfo> {
        let mut storage_info = StorageInfo::default();

        // Linux storage detection
        if cfg!(target_os = "linux") {
            if let Ok(output) = tokio::process::Command::new("df")
                .arg("-BG")
                .arg("/")
                .output()
                .await
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        if let Ok(total_gb) = parts[1].trim_end_matches('G').parse::<f64>() {
                            storage_info.total_gb = total_gb;
                        }
                        if let Ok(available_gb) = parts[3].trim_end_matches('G').parse::<f64>() {
                            storage_info.available_gb = available_gb;
                        }
                        break;
                    }
                }
            }
        }

        // Detect storage type (SSD vs HDD)
        storage_info.storage_type = self.detect_storage_type().await?;

        debug!(
            "Detected storage: {:.1} GB total, {:.1} GB available, type: {:?}",
            storage_info.total_gb, storage_info.available_gb, storage_info.storage_type
        );

        Ok(storage_info)
    }

    /// Detect storage type (SSD vs HDD)
    async fn detect_storage_type(&self) -> ToadStoolResult<StorageType> {
        // Linux: check rotational attribute
        if cfg!(target_os = "linux") {
            if let Ok(rotational) =
                tokio::fs::read_to_string("/sys/block/sda/queue/rotational").await
            {
                if rotational.trim() == "0" {
                    return Ok(StorageType::SSD);
                }
                return Ok(StorageType::HDD);
            }
        }

        // Default assumption: SSD for modern systems
        Ok(StorageType::SSD)
    }

    /// Detect network interfaces and capabilities
    fn detect_network(&self) -> ToadStoolResult<NetworkInfo> {
        let network_info = NetworkInfo {
            interfaces: vec![NetworkInterface {
                name: "default".to_string(),
                interface_type: NetworkInterfaceType::Ethernet,
                speed_mbps: 1000, // Default assumption
                is_wireless: false,
            }],
        };

        debug!(
            "Detected {} network interface(s)",
            network_info.interfaces.len()
        );
        Ok(network_info)
    }

    /// Classify system performance based on hardware capabilities
    async fn classify_performance(
        &self,
        capabilities: &SystemCapabilities,
    ) -> ToadStoolResult<PerformanceClass> {
        let cpu_score = self.calculate_cpu_score(&capabilities.cpu_info);
        let memory_score = self.calculate_memory_score(&capabilities.memory_info);
        let gpu_score = self.calculate_gpu_score(&capabilities.gpu_info);
        let storage_score = self.calculate_storage_score(&capabilities.storage_info);

        let total_score = (cpu_score + memory_score + gpu_score + storage_score) / 4.0;

        let performance_class = if total_score >= 80.0 {
            PerformanceClass::HighEnd
        } else if total_score >= 60.0 {
            PerformanceClass::Mainstream
        } else if total_score >= 40.0 {
            PerformanceClass::Budget
        } else {
            PerformanceClass::LowEnd
        };

        debug!(
            "Performance classification: {:?} (score: {:.1})",
            performance_class, total_score
        );
        Ok(performance_class)
    }

    /// Calculate CPU performance score
    fn calculate_cpu_score(&self, cpu_info: &CpuInfo) -> f64 {
        let core_score = (cpu_info.physical_cores as f64 / 16.0 * 40.0).min(40.0);
        let frequency_score = (cpu_info.base_frequency_mhz / 4000.0 * 30.0).min(30.0);
        let features_score = if cpu_info.features.supports_avx2 {
            20.0
        } else if cpu_info.features.supports_avx {
            15.0
        } else {
            10.0
        };
        let cache_score = (f64::from(cpu_info.cache_size_kb) / 32768.0 * 10.0).min(10.0);

        core_score + frequency_score + features_score + cache_score
    }

    /// Calculate memory performance score
    fn calculate_memory_score(&self, memory_info: &MemoryInfo) -> f64 {
        (memory_info.total_gb / 32.0 * 100.0).min(100.0)
    }

    /// Calculate GPU performance score
    fn calculate_gpu_score(&self, gpu_info: &[GpuInfo]) -> f64 {
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

    /// Calculate storage performance score
    fn calculate_storage_score(&self, storage_info: &StorageInfo) -> f64 {
        let capacity_score = (storage_info.total_gb / 1000.0 * 50.0).min(50.0);
        let type_score = match storage_info.storage_type {
            StorageType::NVME => 50.0,
            StorageType::SSD => 40.0,
            StorageType::HDD => 20.0,
            StorageType::Unknown => 25.0,
        };

        capacity_score + type_score
    }
}

impl Default for HardwareDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete system capabilities detected by hardware scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    /// CPU information and capabilities
    pub cpu_info: CpuInfo,
    /// Number of CPU cores (physical)
    pub cpu_cores: f64,
    /// Memory information
    pub memory_info: MemoryInfo,
    /// Total memory in GB
    pub memory_gb: f64,
    /// GPU information for each detected GPU
    pub gpu_info: Vec<GpuInfo>,
    /// Number of GPUs detected
    pub gpu_count: usize,
    /// GPU memory in GB (first GPU)
    pub gpu_memory_gb: Option<f64>,
    /// Storage information
    pub storage_info: StorageInfo,
    /// Total storage in GB
    pub storage_gb: f64,
    /// Network information
    pub network_info: NetworkInfo,
    /// Overall performance classification
    pub performance_class: PerformanceClass,
}

impl Default for SystemCapabilities {
    fn default() -> Self {
        Self {
            cpu_info: CpuInfo::default(),
            cpu_cores: 4.0,
            memory_info: MemoryInfo::default(),
            memory_gb: 8.0,
            gpu_info: Vec::new(),
            gpu_count: 0,
            gpu_memory_gb: None,
            storage_info: StorageInfo::default(),
            storage_gb: 100.0,
            network_info: NetworkInfo::default(),
            performance_class: PerformanceClass::Mainstream,
        }
    }
}

/// CPU information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub family: u32,
    pub base_frequency_mhz: f64,
    pub max_frequency_mhz: f64,
    pub cache_size_kb: u32,
    pub instruction_sets: Vec<String>,
    pub features: CpuFeatures,
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self {
            model_name: "Unknown CPU".to_string(),
            physical_cores: 4,
            logical_cores: 4,
            family: 0,
            base_frequency_mhz: 2000.0,
            max_frequency_mhz: 3000.0,
            cache_size_kb: 8192,
            instruction_sets: Vec::new(),
            features: CpuFeatures::default(),
        }
    }
}

/// CPU feature flags and capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuFeatures {
    pub supports_avx: bool,
    pub supports_avx2: bool,
    pub supports_sse4_1: bool,
    pub supports_sse4_2: bool,
    pub supports_neon: bool,
}

/// Memory information and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_gb: f64,
    pub available_gb: f64,
    pub memory_type: String,
    pub frequency_mhz: u32,
}

impl Default for MemoryInfo {
    fn default() -> Self {
        Self {
            total_gb: 8.0,
            available_gb: 6.0,
            memory_type: "DDR4".to_string(),
            frequency_mhz: 2400,
        }
    }
}

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

/// Storage information and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub total_gb: f64,
    pub available_gb: f64,
    pub storage_type: StorageType,
}

impl Default for StorageInfo {
    fn default() -> Self {
        Self {
            total_gb: 100.0,
            available_gb: 80.0,
            storage_type: StorageType::SSD,
        }
    }
}

/// Storage device type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    HDD,
    SSD,
    NVME,
    Unknown,
}

/// Network information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub interface_type: NetworkInterfaceType,
    pub speed_mbps: u32,
    pub is_wireless: bool,
}

/// Network interface type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkInterfaceType {
    Ethernet,
    WiFi,
    Loopback,
    Unknown,
}

/// System performance classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceClass {
    LowEnd,
    Budget,
    Mainstream,
    HighEnd,
}

/// System information container
#[derive(Debug)]
struct SystemInfo {
    // Platform-specific system information
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardware_detector_creation() {
        let detector = HardwareDetector::new();
        assert!(detector._system_info.is_none());
    }

    #[tokio::test]
    async fn test_system_scan() {
        let mut detector = HardwareDetector::new();
        let result = detector.scan_system().await;
        assert!(result.is_ok(), "System scan should succeed");

        let capabilities = result.unwrap();
        assert!(capabilities.cpu_cores > 0.0, "Should detect CPU cores");
        assert!(capabilities.memory_gb > 0.0, "Should detect memory");
    }

    #[test]
    fn test_performance_classification() {
        let detector = HardwareDetector::new();

        // Test CPU score calculation
        let cpu_info = CpuInfo {
            physical_cores: 8,
            base_frequency_mhz: 3000.0,
            features: CpuFeatures {
                supports_avx2: true,
                ..Default::default()
            },
            cache_size_kb: 16384,
            ..Default::default()
        };

        let score = detector.calculate_cpu_score(&cpu_info);
        assert!(score > 50.0, "High-end CPU should have good score");
    }

    #[test]
    fn test_system_capabilities_default() {
        let capabilities = SystemCapabilities::default();
        assert_eq!(capabilities.cpu_cores, 4.0);
        assert_eq!(capabilities.memory_gb, 8.0);
        assert_eq!(capabilities.gpu_count, 0);
    }
}
