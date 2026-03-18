// SPDX-License-Identifier: AGPL-3.0-or-later
//! Backend selection logic for intelligent workload routing
//!
//! The `BackendSelector` analyzes workload characteristics and available hardware
//! to select the optimal execution backend. This is the core of ToadStool's
//! workload-centric (not hardware-centric) architecture.

use std::fmt;

use super::{
    analyzer::{ComputeIntensity, GpuAdvantage, ParallelismLevel, WorkloadCharacteristics},
    cuda::CudaBackend,
};

/// Available hardware capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareCapabilities {
    /// CPU cores available
    pub cpu_cores: usize,

    /// Total RAM in bytes
    pub ram_bytes: u64,

    /// GPU devices available
    pub gpu_devices: Vec<GpuDevice>,

    /// CUDA compute capability (if NVIDIA GPU present)
    pub cuda_compute_capability: Option<String>,
}

impl Default for HardwareCapabilities {
    fn default() -> Self {
        Self {
            cpu_cores: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            ram_bytes: Self::detect_ram(),
            gpu_devices: Vec::new(),
            cuda_compute_capability: None,
        }
    }
}

impl HardwareCapabilities {
    /// Create new hardware capabilities with runtime detection
    #[must_use]
    pub fn detect() -> Self {
        Self::default()
    }

    /// Create capabilities with custom values (for testing)
    #[must_use]
    pub const fn new(cpu_cores: usize, ram_bytes: u64) -> Self {
        Self {
            cpu_cores,
            ram_bytes,
            gpu_devices: Vec::new(),
            cuda_compute_capability: None,
        }
    }

    /// Add GPU device
    #[must_use]
    pub fn with_gpu(mut self, device: GpuDevice) -> Self {
        self.gpu_devices.push(device);
        self
    }

    /// Set CUDA compute capability
    #[must_use]
    pub fn with_cuda_capability(mut self, capability: String) -> Self {
        self.cuda_compute_capability = Some(capability);
        self
    }

    /// Check if any GPU is available
    #[must_use]
    pub fn has_gpu(&self) -> bool {
        !self.gpu_devices.is_empty()
    }

    /// Check if NVIDIA GPU with CUDA is available
    #[must_use]
    pub fn has_cuda(&self) -> bool {
        self.cuda_compute_capability.is_some()
            && self
                .gpu_devices
                .iter()
                .any(|d| matches!(d.vendor, GpuVendor::Nvidia))
    }

    /// Get total GPU memory in bytes
    #[must_use]
    pub fn total_gpu_memory(&self) -> u64 {
        self.gpu_devices.iter().map(|d| d.memory_bytes).sum()
    }

    /// Detect system RAM
    const fn detect_ram() -> u64 {
        // Platform-specific RAM detection would go here
        // For now, return a reasonable default
        8 * 1024 * 1024 * 1024 // 8GB default
    }
}

/// GPU device information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDevice {
    /// Device name
    pub name: String,

    /// GPU vendor
    pub vendor: GpuVendor,

    /// Memory in bytes
    pub memory_bytes: u64,

    /// Compute units
    pub compute_units: u32,

    /// Maximum work group size
    pub max_work_group_size: u32,
}

impl GpuDevice {
    /// Create new GPU device
    #[must_use]
    pub const fn new(
        name: String,
        vendor: GpuVendor,
        memory_bytes: u64,
        compute_units: u32,
    ) -> Self {
        Self {
            name,
            vendor,
            memory_bytes,
            compute_units,
            max_work_group_size: 1024, // Common default
        }
    }
}

/// GPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuVendor {
    /// NVIDIA
    Nvidia,

    /// AMD
    Amd,

    /// Intel
    Intel,

    /// Apple
    Apple,

    /// Other/Unknown
    Other,
}

impl fmt::Display for GpuVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nvidia => write!(f, "NVIDIA"),
            Self::Amd => write!(f, "AMD"),
            Self::Intel => write!(f, "Intel"),
            Self::Apple => write!(f, "Apple"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// Backend selection decision
#[derive(Debug, Clone, PartialEq)]
pub struct BackendDecision {
    /// Selected backend for CUDA workloads
    pub cuda_backend: CudaBackend,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,

    /// Reasoning for selection
    pub reasoning: String,

    /// Alternative backends (fallback options)
    pub alternatives: Vec<CudaBackend>,
}

impl BackendDecision {
    /// Create new backend decision
    #[must_use]
    pub const fn new(cuda_backend: CudaBackend, confidence: f64, reasoning: String) -> Self {
        Self {
            cuda_backend,
            confidence,
            reasoning,
            alternatives: Vec::new(),
        }
    }

    /// Add alternative backend
    #[must_use]
    pub fn with_alternative(mut self, backend: CudaBackend) -> Self {
        self.alternatives.push(backend);
        self
    }
}

/// Backend selector for intelligent routing
pub struct BackendSelector {
    /// Cached hardware capabilities
    hardware: HardwareCapabilities,
}

impl BackendSelector {
    /// Create new backend selector with runtime hardware detection
    #[must_use]
    pub fn new() -> Self {
        Self {
            hardware: HardwareCapabilities::detect(),
        }
    }

    /// Create selector with specific hardware capabilities (for testing)
    #[must_use]
    pub const fn with_hardware(hardware: HardwareCapabilities) -> Self {
        Self { hardware }
    }

    /// Select optimal backend for CUDA workload
    #[must_use]
    pub fn select_cuda_backend(
        &self,
        characteristics: &WorkloadCharacteristics,
    ) -> BackendDecision {
        // Decision tree for backend selection

        // 1. Check if NVIDIA GPU with CUDA is available and workload benefits
        if self.hardware.has_cuda() && self.should_use_native_cuda(characteristics) {
            return BackendDecision::new(
                CudaBackend::NativeNvidia,
                0.95,
                "Native CUDA on NVIDIA GPU - optimal performance".to_string(),
            )
            .with_alternative(CudaBackend::TranslatedGpu)
            .with_alternative(CudaBackend::CpuParallel);
        }

        // 2. Check if non-NVIDIA GPU available and workload suitable for translation
        if self.hardware.has_gpu() && self.should_use_translated_gpu(characteristics) {
            let gpu_vendor = self.hardware.gpu_devices[0].vendor;
            return BackendDecision::new(
                CudaBackend::TranslatedGpu,
                0.80,
                format!("CUDA → GPU translation ({gpu_vendor}) - good performance"),
            )
            .with_alternative(CudaBackend::CpuParallel)
            .with_alternative(CudaBackend::CpuSequential);
        }

        // 3. Check if CPU parallelization is viable
        if characteristics.cpu_viable && self.should_use_cpu_parallel(characteristics) {
            return BackendDecision::new(
                CudaBackend::CpuParallel,
                0.60,
                format!(
                    "Multi-threaded CPU ({} cores) - acceptable performance",
                    self.hardware.cpu_cores
                ),
            )
            .with_alternative(CudaBackend::CpuSequential);
        }

        // 4. Fallback to sequential CPU
        BackendDecision::new(
            CudaBackend::CpuSequential,
            0.30,
            "Sequential CPU fallback - limited performance".to_string(),
        )
    }

    /// Should use native CUDA?
    const fn should_use_native_cuda(&self, chars: &WorkloadCharacteristics) -> bool {
        // Always use native CUDA if available, unless workload is trivial
        !matches!(chars.compute_intensity, ComputeIntensity::Minimal)
    }

    /// Should use translated GPU?
    fn should_use_translated_gpu(&self, chars: &WorkloadCharacteristics) -> bool {
        // Use GPU translation if:
        // 1. GPU advantage is significant
        // 2. Memory requirements fit in GPU
        // 3. Parallelism level is high enough

        let _gpu_memory = self.hardware.total_gpu_memory(); // Reserved for future memory checks
        let memory_fits = chars.memory_requirement as u8 <= 3; // Medium or less

        let significant_advantage = matches!(
            chars.gpu_advantage,
            GpuAdvantage::Significant
                | GpuAdvantage::High
                | GpuAdvantage::VeryHigh
                | GpuAdvantage::Critical
        );

        let high_parallelism = matches!(
            chars.parallelism_level,
            ParallelismLevel::High | ParallelismLevel::VeryHigh
        );

        memory_fits && (significant_advantage || high_parallelism)
    }

    /// Should use CPU parallel?
    const fn should_use_cpu_parallel(&self, chars: &WorkloadCharacteristics) -> bool {
        // Use CPU parallel if:
        // 1. Workload has decent parallelism
        // 2. Compute intensity not too high
        // 3. Multiple cores available

        let has_cores = self.hardware.cpu_cores >= 4;

        let decent_parallelism = !matches!(chars.parallelism_level, ParallelismLevel::Sequential);

        let not_extreme = !matches!(
            chars.compute_intensity,
            ComputeIntensity::VeryHigh | ComputeIntensity::Extreme
        );

        has_cores && decent_parallelism && not_extreme
    }

    /// Get hardware capabilities
    #[must_use]
    pub const fn hardware(&self) -> &HardwareCapabilities {
        &self.hardware
    }
}

impl Default for BackendSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workload::analyzer::MemoryRequirement;

    #[test]
    fn test_hardware_detection() {
        let hw = HardwareCapabilities::detect();
        assert!(hw.cpu_cores > 0);
        assert!(hw.ram_bytes > 0);
    }

    #[test]
    fn test_nvidia_gpu_native_cuda() {
        let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024)
            .with_gpu(GpuDevice::new(
                "RTX 3090".to_string(),
                GpuVendor::Nvidia,
                24 * 1024 * 1024 * 1024,
                128,
            ))
            .with_cuda_capability("8.6".to_string());

        let selector = BackendSelector::with_hardware(hw);

        let chars = WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::High,
            memory_requirement: MemoryRequirement::Medium,
            parallelism_level: ParallelismLevel::VeryHigh,
            gpu_advantage: GpuAdvantage::VeryHigh,
            cpu_viable: true,
            estimated_flops: Some(1_000_000_000),
        };

        let decision = selector.select_cuda_backend(&chars);
        assert_eq!(decision.cuda_backend, CudaBackend::NativeNvidia);
        assert!(decision.confidence > 0.9);
    }

    #[test]
    fn test_amd_gpu_translation() {
        let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024).with_gpu(GpuDevice::new(
            "RX 7900 XTX".to_string(),
            GpuVendor::Amd,
            24 * 1024 * 1024 * 1024,
            96,
        ));

        let selector = BackendSelector::with_hardware(hw);

        let chars = WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::High,
            memory_requirement: MemoryRequirement::Medium,
            parallelism_level: ParallelismLevel::High,
            gpu_advantage: GpuAdvantage::High,
            cpu_viable: true,
            estimated_flops: Some(500_000_000),
        };

        let decision = selector.select_cuda_backend(&chars);
        assert_eq!(decision.cuda_backend, CudaBackend::TranslatedGpu);
        assert!(decision.confidence > 0.7);
        assert!(decision.reasoning.contains("AMD"));
    }

    #[test]
    fn test_cpu_parallel_fallback() {
        let hw = HardwareCapabilities::new(16, 32 * 1024 * 1024 * 1024); // No GPU

        let selector = BackendSelector::with_hardware(hw);

        let chars = WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::Medium,
            memory_requirement: MemoryRequirement::Small,
            parallelism_level: ParallelismLevel::Medium,
            gpu_advantage: GpuAdvantage::Moderate,
            cpu_viable: true,
            estimated_flops: Some(100_000_000),
        };

        let decision = selector.select_cuda_backend(&chars);
        assert_eq!(decision.cuda_backend, CudaBackend::CpuParallel);
        assert!(decision.reasoning.contains("16 cores"));
    }

    #[test]
    fn test_cpu_sequential_last_resort() {
        let hw = HardwareCapabilities::new(2, 4 * 1024 * 1024 * 1024); // Limited CPU, no GPU

        let selector = BackendSelector::with_hardware(hw);

        let chars = WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::Extreme,
            memory_requirement: MemoryRequirement::Huge,
            parallelism_level: ParallelismLevel::Sequential,
            gpu_advantage: GpuAdvantage::Critical,
            cpu_viable: false,
            estimated_flops: Some(10_000_000_000),
        };

        let decision = selector.select_cuda_backend(&chars);
        assert_eq!(decision.cuda_backend, CudaBackend::CpuSequential);
        assert!(decision.confidence < 0.5);
    }

    #[test]
    fn test_apple_gpu_translation() {
        let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024).with_gpu(GpuDevice::new(
            "Apple M2 Max".to_string(),
            GpuVendor::Apple,
            32 * 1024 * 1024 * 1024,
            38,
        ));

        let selector = BackendSelector::with_hardware(hw);

        let chars = WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::High,
            memory_requirement: MemoryRequirement::Medium,
            parallelism_level: ParallelismLevel::High,
            gpu_advantage: GpuAdvantage::Significant,
            cpu_viable: true,
            estimated_flops: None,
        };

        let decision = selector.select_cuda_backend(&chars);
        assert_eq!(decision.cuda_backend, CudaBackend::TranslatedGpu);
        assert!(decision.reasoning.contains("Apple"));
    }

    #[test]
    fn test_alternatives_provided() {
        let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024)
            .with_gpu(GpuDevice::new(
                "RTX 3090".to_string(),
                GpuVendor::Nvidia,
                24 * 1024 * 1024 * 1024,
                128,
            ))
            .with_cuda_capability("8.6".to_string());

        let selector = BackendSelector::with_hardware(hw);

        let chars = WorkloadCharacteristics::default();
        let decision = selector.select_cuda_backend(&chars);

        // Should have alternatives
        assert!(!decision.alternatives.is_empty());
        assert!(decision.alternatives.contains(&CudaBackend::TranslatedGpu));
    }
}
