//! Hardware detection and capability assessment for auto-configuration

use serde::{Deserialize, Serialize};
use sysinfo::System;
use tracing::info;

use toadstool::error::ToadStoolResult;

/// Hardware detection and capability assessment
pub struct HardwareDetector {
    system: System,
}

impl Default for HardwareDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareDetector {
    /// Create new hardware detector
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self { system }
    }

    /// Comprehensive system scan
    pub async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        info!("🔍 Scanning system hardware capabilities...");

        // Refresh system information
        self.system.refresh_all();

        let mut capabilities = SystemCapabilities::default();

        // CPU detection
        capabilities.cpu_cores = self.detect_cpu_cores();
        capabilities.cpu_model = self.detect_cpu_model();
        capabilities.cpu_frequency_mhz = self.detect_cpu_frequency();

        // Memory detection
        capabilities.memory_gb = self.detect_memory_gb();
        capabilities.swap_gb = self.detect_swap_gb();

        // Storage detection
        capabilities.storage_info = self.detect_storage();

        // Network detection
        capabilities.network_interfaces = self.detect_network();

        // Platform-specific detection
        capabilities.platform = self.detect_platform();
        capabilities.architecture = self.detect_architecture();

        // Container runtime detection
        capabilities.has_docker = self.detect_docker().await;
        capabilities.has_podman = self.detect_podman().await;
        capabilities.has_containerd = self.detect_containerd().await;

        // GPU detection
        capabilities.gpu_count = self.detect_gpu_count().await;
        capabilities.gpu_platform = self.detect_gpu_platform().await;
        capabilities.gpu_memory_gb = self.detect_gpu_memory().await;

        // Virtualization detection
        capabilities.is_virtualized = self.detect_virtualization();
        capabilities.virtualization_type = self.detect_virtualization_type();

        info!(
            "✅ Hardware scan complete: {} cores, {:.1}GB RAM, {} GPUs",
            capabilities.cpu_cores, capabilities.memory_gb, capabilities.gpu_count
        );

        Ok(capabilities)
    }

    /// Detect CPU cores
    fn detect_cpu_cores(&self) -> u32 {
        self.system.cpus().len() as u32
    }

    /// Detect CPU model
    fn detect_cpu_model(&self) -> String {
        self.system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string())
    }

    /// Detect CPU frequency
    fn detect_cpu_frequency(&self) -> u64 {
        self.system
            .cpus()
            .first()
            .map(|cpu| cpu.frequency())
            .unwrap_or(0)
    }

    /// Detect total memory in GB
    fn detect_memory_gb(&self) -> f64 {
        self.system.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Detect swap memory in GB
    fn detect_swap_gb(&self) -> f64 {
        self.system.total_swap() as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Detect storage information
    fn detect_storage(&self) -> Vec<StorageInfo> {
        // For now, return empty vec - in a real implementation we'd use platform-specific methods
        // or a different crate that provides disk information
        Vec::new()
    }

    /// Detect network interfaces
    fn detect_network(&self) -> Vec<NetworkInfo> {
        // For now, return empty vec - in a real implementation we'd use platform-specific methods
        // or a different crate that provides network information
        Vec::new()
    }

    /// Detect platform
    fn detect_platform(&self) -> String {
        std::env::consts::OS.to_string()
    }

    /// Detect architecture
    fn detect_architecture(&self) -> String {
        std::env::consts::ARCH.to_string()
    }

    /// Detect Docker availability
    async fn detect_docker(&self) -> bool {
        tokio::process::Command::new("docker")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Detect Podman availability
    async fn detect_podman(&self) -> bool {
        tokio::process::Command::new("podman")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Detect containerd availability
    async fn detect_containerd(&self) -> bool {
        tokio::process::Command::new("ctr")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Detect GPU count
    async fn detect_gpu_count(&self) -> u32 {
        // Fallback: try nvidia-smi
        if let Ok(output) = tokio::process::Command::new("nvidia-smi")
            .arg("--list-gpus")
            .output()
            .await
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return stdout.lines().filter(|line| line.contains("GPU")).count() as u32;
            }
        }

        0
    }

    /// Detect GPU platform
    async fn detect_gpu_platform(&self) -> Option<String> {
        // Try NVIDIA first
        if tokio::process::Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Some("CUDA".to_string());
        }

        // Try AMD
        if tokio::process::Command::new("rocm-smi")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Some("ROCm".to_string());
        }

        None
    }

    /// Detect GPU memory
    async fn detect_gpu_memory(&self) -> Option<f64> {
        // Try nvidia-smi for NVIDIA GPUs
        if let Ok(output) = tokio::process::Command::new("nvidia-smi")
            .arg("--query-gpu=memory.total")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    if let Ok(memory_mb) = line.trim().parse::<f64>() {
                        return Some(memory_mb / 1024.0); // Convert MB to GB
                    }
                }
            }
        }

        None
    }

    /// Detect if running in virtualized environment
    fn detect_virtualization(&self) -> bool {
        // Check common virtualization indicators
        std::fs::read_to_string("/proc/cpuinfo")
            .map(|content| {
                content.contains("hypervisor")
                    || content.contains("QEMU")
                    || content.contains("VirtualBox")
                    || content.contains("VMware")
            })
            .unwrap_or(false)
            || std::fs::read_to_string("/sys/class/dmi/id/product_name")
                .map(|content| {
                    content.contains("VirtualBox")
                        || content.contains("VMware")
                        || content.contains("QEMU")
                })
                .unwrap_or(false)
    }

    /// Detect virtualization type
    fn detect_virtualization_type(&self) -> Option<String> {
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            if content.contains("QEMU") {
                return Some("QEMU/KVM".to_string());
            }
            if content.contains("VirtualBox") {
                return Some("VirtualBox".to_string());
            }
            if content.contains("VMware") {
                return Some("VMware".to_string());
            }
        }

        None
    }

    pub async fn detect_capabilities(&self) -> SystemCapabilities {
        SystemCapabilities {
            cpu_cores: self.detect_cpu_cores(),
            cpu_model: self.detect_cpu_model(),
            cpu_frequency_mhz: self.detect_cpu_frequency(),
            memory_gb: self.detect_memory_gb(),
            swap_gb: self.detect_swap_gb(),
            storage_info: self.detect_storage(),
            network_interfaces: self.detect_network(),
            platform: self.detect_platform(),
            architecture: self.detect_architecture(),
            has_docker: self.detect_docker().await,
            has_podman: self.detect_podman().await,
            has_containerd: self.detect_containerd().await,
            gpu_count: self.detect_gpu_count().await,
            gpu_platform: self.detect_gpu_platform().await,
            gpu_memory_gb: self.detect_gpu_memory().await,
            is_virtualized: self.detect_virtualization(),
            virtualization_type: self.detect_virtualization_type(),
        }
    }
}

/// System capabilities detected by hardware scanner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    // CPU information
    pub cpu_cores: u32,
    pub cpu_model: String,
    pub cpu_frequency_mhz: u64,

    // Memory information
    pub memory_gb: f64,
    pub swap_gb: f64,

    // Storage information
    pub storage_info: Vec<StorageInfo>,

    // Network information
    pub network_interfaces: Vec<NetworkInfo>,

    // Platform information
    pub platform: String,
    pub architecture: String,

    // Container runtime availability
    pub has_docker: bool,
    pub has_podman: bool,
    pub has_containerd: bool,

    // GPU information
    pub gpu_count: u32,
    pub gpu_platform: Option<String>,
    pub gpu_memory_gb: Option<f64>,

    // Virtualization information
    pub is_virtualized: bool,
    pub virtualization_type: Option<String>,
}

impl Default for SystemCapabilities {
    fn default() -> Self {
        Self {
            cpu_cores: 1,
            cpu_model: "Unknown".to_string(),
            cpu_frequency_mhz: 0,
            memory_gb: 1.0,
            swap_gb: 0.0,
            storage_info: Vec::new(),
            network_interfaces: Vec::new(),
            platform: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            has_docker: false,
            has_podman: false,
            has_containerd: false,
            gpu_count: 0,
            gpu_platform: None,
            gpu_memory_gb: None,
            is_virtualized: false,
            virtualization_type: None,
        }
    }
}

/// Storage device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub name: String,
    pub mount_point: String,
    pub total_gb: f64,
    pub available_gb: f64,
    pub file_system: String,
    pub is_removable: bool,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub name: String,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
}

impl SystemCapabilities {
    /// Check if system has sufficient resources for containers
    pub fn has_container_support(&self) -> bool {
        self.cpu_cores >= 2
            && self.memory_gb >= 4.0
            && (self.has_docker || self.has_podman || self.has_containerd)
    }

    /// Check if system has GPU support
    pub fn has_gpu_support(&self) -> bool {
        self.gpu_count > 0 && self.gpu_platform.is_some()
    }

    /// Get recommended performance profile
    pub fn get_recommended_performance_profile(&self) -> super::PerformanceProfile {
        if self.cpu_cores >= 16 && self.memory_gb >= 32.0 {
            super::PerformanceProfile::MaxPerformance
        } else if self.cpu_cores >= 8 && self.memory_gb >= 16.0 {
            super::PerformanceProfile::Performance
        } else if self.cpu_cores >= 4 && self.memory_gb >= 8.0 {
            super::PerformanceProfile::Balanced
        } else {
            super::PerformanceProfile::PowerSaver
        }
    }

    /// Get recommended security profile
    pub fn get_recommended_security_profile(&self) -> super::SecurityProfile {
        if self.is_virtualized {
            // Already in a VM, can be less restrictive
            super::SecurityProfile::Standard
        } else if self.has_container_support() {
            // Can use container isolation
            super::SecurityProfile::High
        } else {
            // Need maximum security without containers
            super::SecurityProfile::Maximum
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardware_detection() {
        let mut detector = HardwareDetector::new();
        let capabilities = detector.scan_system().await.unwrap();

        assert!(
            capabilities.cpu_cores > 0,
            "Should detect at least one CPU core"
        );
        assert!(capabilities.memory_gb > 0.0, "Should detect some memory");
        assert!(!capabilities.platform.is_empty(), "Should detect platform");
        assert!(
            !capabilities.architecture.is_empty(),
            "Should detect architecture"
        );
    }

    #[test]
    fn test_system_capabilities_defaults() {
        let capabilities = SystemCapabilities::default();
        assert_eq!(capabilities.cpu_cores, 1);
        assert_eq!(capabilities.memory_gb, 1.0);
    }

    #[test]
    fn test_container_support_detection() {
        let mut capabilities = SystemCapabilities::default();
        assert!(!capabilities.has_container_support());

        capabilities.cpu_cores = 4;
        capabilities.memory_gb = 8.0;
        capabilities.has_docker = true;
        assert!(capabilities.has_container_support());
    }
}
