//! # Hardware Detection and Capability Assessment
//!
//! Comprehensive system hardware detection for optimal `ToadStool` configuration.
//! Detects CPU, memory, GPU, storage, and network capabilities to enable
//! zero-touch optimization.
//!
//! ## Architecture
//!
//! This module is organized by hardware type:
//! - **cpu**: CPU detection and capabilities
//! - **memory**: Memory detection and configuration
//! - **gpu**: GPU detection and vendor support
//! - **storage**: Storage detection and type classification
//! - **network**: Network interface detection

pub mod cpu;
pub mod memory;
pub mod gpu;
pub mod storage;
pub mod network;

// Re-export all public types for backward compatibility
pub use cpu::*;
pub use memory::*;
pub use gpu::*;
pub use storage::*;
pub use network::*;

use serde::{Deserialize, Serialize};
use tracing::info;

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
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if system information cannot be retrieved
    /// or hardware detection fails.
    #[must_use = "Hardware scan result should be checked"]
    pub async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        info!("🔍 Starting comprehensive hardware scan...");

        let mut capabilities = SystemCapabilities::default();

        // CPU detection
        info!("  🖥️ Scanning CPU capabilities...");
        capabilities.cpu_info = cpu::detect_cpu(self).await?;
        capabilities.cpu_cores = capabilities.cpu_info.physical_cores as f64;

        // Memory detection
        info!("  🧠 Scanning memory configuration...");
        capabilities.memory_info = memory::detect_memory(self).await?;
        capabilities.memory_gb = capabilities.memory_info.total_gb;

        // GPU detection
        info!("  🎮 Scanning GPU capabilities...");
        capabilities.gpu_info = gpu::detect_gpus(self).await?;
        capabilities.gpu_count = capabilities.gpu_info.len();
        capabilities.gpu_memory_gb = capabilities.gpu_info.first().map(|gpu| gpu.memory_gb);

        // Storage detection
        info!("  💾 Scanning storage configuration...");
        capabilities.storage_info = storage::detect_storage(self).await?;
        capabilities.storage_gb = capabilities.storage_info.total_gb;

        // Network detection
        info!("  🌐 Scanning network interfaces...");
        capabilities.network_info = network::detect_network(self)?;

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

    /// Classify system performance based on hardware capabilities
    async fn classify_performance(
        &self,
        capabilities: &SystemCapabilities,
    ) -> ToadStoolResult<PerformanceClass> {
        use tracing::debug;
        
        let cpu_score = cpu::calculate_cpu_score(&capabilities.cpu_info);
        let memory_score = memory::calculate_memory_score(&capabilities.memory_info);
        let gpu_score = gpu::calculate_gpu_score(&capabilities.gpu_info);
        let storage_score = storage::calculate_storage_score(&capabilities.storage_info);

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_hardware_detector_creation() {
        let detector = HardwareDetector::new();
        assert!(detector._system_info.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_scan() {
        let mut detector = HardwareDetector::new();
        let result = detector.scan_system().await;
        assert!(result.is_ok(), "System scan should succeed");

        let capabilities = result.unwrap();
        assert!(capabilities.cpu_cores > 0.0, "Should detect CPU cores");
        assert!(capabilities.memory_gb > 0.0, "Should detect memory");
    }

    #[test]
    fn test_system_capabilities_default() {
        let capabilities = SystemCapabilities::default();
        assert_eq!(capabilities.cpu_cores, 4.0);
        assert_eq!(capabilities.memory_gb, 8.0);
        assert_eq!(capabilities.gpu_count, 0);
    }
}
