//! Unified Hardware Base - Universal Compute Abstraction
//!
//! **Philosophy**: Hardware executes math, math doesn't know hardware
//!
//! This module provides the hardware abstraction layer for BarraCuda:
//! - Trait-based execution (CPU, GPU, TPU, NPU implement same traits)
//! - Runtime capability discovery (query what hardware can do)
//! - Smart scheduling (match operations to best hardware)
//! - Unified memory (zero-copy when possible)
//!
//! **Deep Debt Principles**:
//! - ✅ Hardware agnostic (works with ANY compute device)
//! - ✅ Runtime discovery (detects capabilities at runtime)
//! - ✅ Capability-based (matches workload to hardware)
//! - ✅ Extensible (new hardware = implement trait)

use crate::device::Device;
use crate::error::Result;
use crate::gpu_executor::GpuExecutor;
use crate::unified_math::{MathOp, TensorDescriptor};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

/// Universal compute executor
///
/// **Hardware-agnostic**: Any device that can execute mathematical operations
#[async_trait]
pub trait ComputeExecutor: Send + Sync {
    /// Get executor name (e.g., "NVIDIA RTX 4090", "Google TPU v4")
    fn name(&self) -> &str;

    /// Get hardware type
    fn hardware_type(&self) -> HardwareType;

    /// Get capabilities
    fn capabilities(&self) -> &HardwareCapabilities;

    /// Check if executor can handle this operation
    fn can_execute(&self, op: &MathOp, inputs: &[TensorDescriptor]) -> bool;

    /// Score how well this executor matches the operation (0.0-1.0)
    /// Higher score = better match
    fn score_operation(&self, op: &MathOp, inputs: &[TensorDescriptor]) -> f64;

    /// Execute mathematical operation
    async fn execute(
        &self,
        op: &MathOp,
        inputs: Vec<Arc<dyn TensorStorage>>,
    ) -> Result<Arc<dyn TensorStorage>>;

    /// Allocate tensor storage on this device
    async fn allocate(&self, descriptor: TensorDescriptor) -> Result<Arc<dyn TensorStorage>>;

    /// Transfer tensor to this device
    async fn transfer(&self, tensor: Arc<dyn TensorStorage>) -> Result<Arc<dyn TensorStorage>>;
}

/// Hardware type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardwareType {
    /// CPU (any architecture)
    CPU,

    /// GPU (via WGSL/WebGPU)
    GPU,

    /// TPU (Tensor Processing Unit)
    TPU,

    /// NPU (Neuromorphic Processing Unit)
    NPU,

    /// FPGA (Field-Programmable Gate Array)
    FPGA,

    /// ASIC (Application-Specific Integrated Circuit)
    ASIC,

    /// Custom/Unknown
    Custom,
}

/// Hardware capabilities descriptor
#[derive(Debug, Clone)]
pub struct HardwareCapabilities {
    /// Hardware type
    pub hardware_type: HardwareType,

    /// Parallel execution support
    pub parallelism: ParallelismCapabilities,

    /// Memory capabilities
    pub memory: MemoryCapabilities,

    /// Precision support
    pub precision: PrecisionCapabilities,

    /// Supported operations
    pub operations: OperationCapabilities,

    /// Performance characteristics
    pub performance: PerformanceCapabilities,
}

/// Parallelism capabilities
#[derive(Debug, Clone)]
pub struct ParallelismCapabilities {
    /// Maximum parallel threads/units
    pub max_parallel_units: usize,

    /// SIMD width (1 = scalar, 4 = SSE, 8 = AVX, etc.)
    pub simd_width: usize,

    /// Supports task parallelism
    pub task_parallel: bool,

    /// Supports data parallelism
    pub data_parallel: bool,

    /// Supports pipeline parallelism
    pub pipeline_parallel: bool,
}

/// Memory capabilities
#[derive(Debug, Clone)]
pub struct MemoryCapabilities {
    /// Total memory (bytes)
    pub total_bytes: u64,

    /// Available memory (bytes)
    pub available_bytes: u64,

    /// Memory bandwidth (bytes/sec)
    pub bandwidth_bytes_per_sec: u64,

    /// Supports unified memory
    pub unified_memory: bool,

    /// Supports zero-copy
    pub zero_copy: bool,
}

/// Precision capabilities
#[derive(Debug, Clone)]
pub struct PrecisionCapabilities {
    /// Supports FP16
    pub fp16: bool,

    /// Supports FP32
    pub fp32: bool,

    /// Supports FP64
    pub fp64: bool,

    /// Supports INT8
    pub int8: bool,

    /// Supports INT16
    pub int16: bool,

    /// Supports INT32
    pub int32: bool,

    /// Supports INT64
    pub int64: bool,

    /// Supports mixed precision
    pub mixed_precision: bool,
}

/// Operation capabilities
#[derive(Debug, Clone)]
pub struct OperationCapabilities {
    /// Supports matrix multiply
    pub matmul: bool,

    /// Supports convolution
    pub convolution: bool,

    /// Supports FFT
    pub fft: bool,

    /// Supports reductions
    pub reductions: bool,

    /// Supports sparse operations
    pub sparse: bool,

    /// Supports custom kernels
    pub custom_kernels: bool,
}

/// Performance characteristics
#[derive(Debug, Clone)]
pub struct PerformanceCapabilities {
    /// Peak TFLOPS (FP32)
    pub peak_tflops_fp32: f64,

    /// Peak TFLOPS (FP16)
    pub peak_tflops_fp16: f64,

    /// Peak memory bandwidth (GB/s)
    pub peak_bandwidth_gbps: f64,

    /// Typical power consumption (watts)
    pub typical_power_watts: f64,

    /// Latency (microseconds for small operations)
    pub typical_latency_us: f64,
}

/// Tensor storage abstraction
///
/// **Hardware-agnostic**: Data can live on any device
#[async_trait]
pub trait TensorStorage: Send + Sync {
    /// Get tensor descriptor
    fn descriptor(&self) -> &TensorDescriptor;

    /// Get hardware type where data resides
    fn hardware_type(&self) -> HardwareType;

    /// Read data to CPU memory
    async fn read_to_cpu(&self) -> Result<Vec<u8>>;

    /// Write data from CPU memory
    async fn write_from_cpu(&mut self, data: &[u8]) -> Result<()>;

    /// Check if data is on CPU
    fn is_cpu(&self) -> bool {
        self.hardware_type() == HardwareType::CPU
    }

    /// Check if data is on GPU
    fn is_gpu(&self) -> bool {
        self.hardware_type() == HardwareType::GPU
    }

    /// Check if data is on TPU
    fn is_tpu(&self) -> bool {
        self.hardware_type() == HardwareType::TPU
    }

    /// Return the underlying `Arc<wgpu::Buffer>` if this storage lives on a wgpu GPU.
    ///
    /// Default: `None` (CPU and NPU storage return nothing).
    /// `GpuTensorStorage` overrides this to enable zero-copy input paths —
    /// callers can skip the GPU→CPU→GPU round-trip when the buffer is already
    /// on the target device.
    fn as_wgpu_buffer(&self) -> Option<Arc<::wgpu::Buffer>> {
        None
    }
}

/// Compute scheduler
///
/// Selects best hardware for each operation
pub struct ComputeScheduler {
    /// Available executors
    executors: Vec<Arc<dyn ComputeExecutor>>,
}

impl ComputeScheduler {
    /// Create new scheduler with available executors
    pub fn new(executors: Vec<Arc<dyn ComputeExecutor>>) -> Self {
        Self { executors }
    }

    /// Select best executor for operation
    pub fn select_executor(
        &self,
        op: &MathOp,
        inputs: &[TensorDescriptor],
    ) -> Option<Arc<dyn ComputeExecutor>> {
        self.executors
            .iter()
            .filter(|e| e.can_execute(op, inputs))
            .max_by(|a, b| {
                let score_a = a.score_operation(op, inputs);
                let score_b = b.score_operation(op, inputs);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Execute operation on best available hardware
    pub async fn execute(
        &self,
        op: &MathOp,
        inputs: Vec<Arc<dyn TensorStorage>>,
    ) -> Result<Arc<dyn TensorStorage>> {
        // Get descriptors
        let descriptors: Vec<_> = inputs.iter().map(|t| t.descriptor().clone()).collect();

        // Select executor
        let executor = self.select_executor(op, &descriptors).ok_or_else(|| {
            crate::error::BarracudaError::NoAvailableExecutor {
                operation: format!("{:?}", op),
            }
        })?;

        // Execute
        executor.execute(op, inputs).await
    }
}

/// Hardware discovery
pub struct HardwareDiscovery;

impl HardwareDiscovery {
    /// Discover all available compute hardware
    pub async fn discover_all() -> Result<Vec<Arc<dyn ComputeExecutor>>> {
        let mut executors: Vec<Arc<dyn ComputeExecutor>> = Vec::new();

        // CPU is always available
        executors.push(Arc::new(CpuExecutor::new()));

        // Discover GPUs (via wgpu)
        if let Ok(gpu_executors) = Self::discover_gpus().await {
            executors.extend(gpu_executors);
        }

        // Discover TPUs
        #[cfg(feature = "tpu")]
        if let Ok(tpu_executors) = Self::discover_tpus().await {
            executors.extend(tpu_executors);
        }

        // Discover NPUs
        if let Ok(npu_executors) = Self::discover_npus().await {
            executors.extend(npu_executors);
        }

        Ok(executors)
    }

    async fn discover_gpus() -> Result<Vec<Arc<dyn ComputeExecutor>>> {
        // Use GpuExecutor which wraps WgpuDevice and implements ComputeExecutor
        let available = Device::GPU.is_available();
        debug!("GPU discovery: available={}", available);

        if !available {
            return Ok(Vec::new());
        }

        match GpuExecutor::new().await {
            Ok(executor) => {
                debug!("GPU discovered: {}", executor.name());
                Ok(vec![Arc::new(executor) as Arc<dyn ComputeExecutor>])
            }
            Err(e) => {
                debug!("GPU discovery failed: {}", e);
                Ok(Vec::new())
            }
        }
    }

    #[cfg(feature = "tpu")]
    async fn discover_tpus() -> Result<Vec<Arc<dyn ComputeExecutor>>> {
        // TPU access currently uses TpuDevice directly via device/tpu.rs
        // This executor wrapper enables unified dispatching when TPU hardware is available
        // Note: TPU feature requires external hardware setup (Google Cloud or Edge TPU)
        debug!("TPU discovery: hardware not available in this environment");
        Ok(Vec::new())
    }

    async fn discover_npus() -> Result<Vec<Arc<dyn ComputeExecutor>>> {
        // NPU discovery using NpuExecutor (wraps AkidaExecutor)
        match crate::npu_executor::NpuExecutor::new() {
            Ok(executor) => {
                debug!(
                    "NPU discovered: {} with {} NPUs",
                    executor.name(),
                    executor.npu_count()
                );
                Ok(vec![Arc::new(executor) as Arc<dyn ComputeExecutor>])
            }
            Err(e) => {
                debug!("NPU discovery failed (no Akida hardware): {}", e);
                Ok(Vec::new())
            }
        }
    }
}

/// CPU executor implementation (always available)
///
/// **Deep Debt Principles**:
/// - ✅ Runtime CPU discovery via `std::thread::available_parallelism()` (pure Rust)
/// - ✅ Memory/bandwidth are *conservative estimates* (barracuda doesn't depend on sysinfo)
/// - ✅ Same WGSL shaders run via WGPU software rasterizer (llvmpipe)
///
/// **Memory Note**: Values below are conservative fallbacks. Higher-level code (e.g., toadstool)
/// that has sysinfo can override with actual system info when constructing compute pools.
/// This was proven in cross_hardware_parity tests (Feb 8, 2026).
struct CpuExecutor {
    capabilities: HardwareCapabilities,
}

impl CpuExecutor {
    fn new() -> Self {
        // Query actual core count (pure Rust - no sysinfo dependency)
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        // SIMD width: Runtime detection via CPUID (Deep Debt evolution)
        // Using runtime detection instead of compile-time cfg!() for accurate width
        #[cfg(target_arch = "x86_64")]
        let simd_width = {
            if std::arch::is_x86_feature_detected!("avx512f") {
                16 // AVX-512: 512-bit = 16 × f32
            } else if std::arch::is_x86_feature_detected!("avx2") {
                8 // AVX2: 256-bit = 8 × f32
            } else if std::arch::is_x86_feature_detected!("sse4.1") {
                4 // SSE4: 128-bit = 4 × f32
            } else {
                4 // Baseline SSE2 (guaranteed on x86_64)
            }
        };

        #[cfg(target_arch = "aarch64")]
        let simd_width = 4; // NEON is 128-bit = 4 × f32 (always available on aarch64)

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let simd_width = 4; // Conservative fallback for other architectures

        Self {
            capabilities: HardwareCapabilities {
                hardware_type: HardwareType::CPU,
                parallelism: ParallelismCapabilities {
                    max_parallel_units: cpu_cores,
                    simd_width,
                    task_parallel: true,
                    data_parallel: true,
                    pipeline_parallel: true,
                },
                memory: MemoryCapabilities {
                    // Conservative estimates - barracuda is a compute library
                    // without sysinfo dependency. Callers can provide actual
                    // values via capability overrides if needed.
                    total_bytes: 16 * 1024 * 1024 * 1024, // 16GB fallback
                    available_bytes: 8 * 1024 * 1024 * 1024, // 8GB fallback
                    bandwidth_bytes_per_sec: 50 * 1024 * 1024 * 1024, // 50 GB/s (DDR4-3200)
                    unified_memory: true,
                    zero_copy: true,
                },
                precision: PrecisionCapabilities {
                    fp16: false, // CPU doesn't have native fp16
                    fp32: true,
                    fp64: true,
                    int8: true,
                    int16: true,
                    int32: true,
                    int64: true,
                    mixed_precision: false,
                },
                operations: OperationCapabilities {
                    matmul: true,
                    convolution: true,
                    fft: true,
                    reductions: true,
                    sparse: true,
                    custom_kernels: false,
                },
                performance: PerformanceCapabilities {
                    // Estimates based on typical modern CPU (8-core)
                    // Actual: ~100 GFLOPS per core for AVX2 FMA
                    peak_tflops_fp32: (cpu_cores as f64 * 0.1).min(2.0),
                    peak_tflops_fp16: 0.0, // No native fp16
                    peak_bandwidth_gbps: 50.0,
                    typical_power_watts: 65.0,
                    typical_latency_us: 10.0,
                },
            },
        }
    }
}

#[async_trait]
impl ComputeExecutor for CpuExecutor {
    fn name(&self) -> &str {
        "CPU (Native)"
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::CPU
    }

    fn capabilities(&self) -> &HardwareCapabilities {
        &self.capabilities
    }

    fn can_execute(&self, _op: &MathOp, _inputs: &[TensorDescriptor]) -> bool {
        // CPU can execute everything via WGPU software rasterizer
        true
    }

    fn score_operation(&self, _op: &MathOp, _inputs: &[TensorDescriptor]) -> f64 {
        // CPU baseline score: 0.5
        // GPU scores higher for parallel ops
        0.5
    }

    async fn execute(
        &self,
        op: &MathOp,
        inputs: Vec<Arc<dyn TensorStorage>>,
    ) -> Result<Arc<dyn TensorStorage>> {
        if inputs.is_empty() {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: "No inputs provided".to_string(),
            });
        }
        // Delegate to the standalone CpuExecutor which already handles MathOp dispatch
        let standalone = crate::cpu_executor::CpuExecutor::new();
        standalone.execute(op, inputs).await
    }

    async fn allocate(&self, descriptor: TensorDescriptor) -> Result<Arc<dyn TensorStorage>> {
        // CPU allocation is straightforward - just a Vec<u8>
        let byte_size = descriptor.numel * descriptor.dtype.size_bytes();
        Ok(Arc::new(CpuTensorStorageSimple {
            descriptor,
            data: vec![0u8; byte_size],
        }))
    }

    async fn transfer(&self, tensor: Arc<dyn TensorStorage>) -> Result<Arc<dyn TensorStorage>> {
        if tensor.is_cpu() {
            Ok(tensor)
        } else {
            // Read from device to CPU
            let data = tensor.read_to_cpu().await?;
            let descriptor = tensor.descriptor().clone();
            let mut cpu_tensor = CpuTensorStorageSimple {
                descriptor,
                data: vec![0u8; data.len()],
            };
            cpu_tensor.data = data;
            Ok(Arc::new(cpu_tensor))
        }
    }
}

/// Simple CPU tensor storage for the scheduler path
struct CpuTensorStorageSimple {
    descriptor: TensorDescriptor,
    data: Vec<u8>,
}

#[async_trait]
impl TensorStorage for CpuTensorStorageSimple {
    fn descriptor(&self) -> &TensorDescriptor {
        &self.descriptor
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::CPU
    }

    async fn read_to_cpu(&self) -> Result<Vec<u8>> {
        Ok(self.data.clone())
    }

    async fn write_from_cpu(&mut self, data: &[u8]) -> Result<()> {
        if data.len() != self.data.len() {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "Data size mismatch: expected {}, got {}",
                    self.data.len(),
                    data.len()
                ),
            });
        }
        self.data.copy_from_slice(data);
        Ok(())
    }
}

// =============================================================================
// Mixed-Hardware Infrastructure (M-009) — MixedSubstrate, TransferCost, PcieBridge
// =============================================================================

/// Mixed dispatch substrate — cross-device routing for GPU ↔ NPU ↔ CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixedSubstrate {
    GpuOnly,
    CpuOnly,
    NpuOnly,
    GpuToCpu,
    CpuToGpu,
    GpuToNpu,
    NpuToGpu,
}

/// Estimated transfer cost for cross-device data movement.
#[derive(Debug, Clone, Copy)]
pub struct TransferCost {
    pub latency_us: f64,
    pub bandwidth_gbps: f64,
}

impl TransferCost {
    /// Estimate total transfer time in microseconds for given byte count.
    #[must_use]
    pub fn estimated_us(&self, bytes: usize) -> f64 {
        self.latency_us + (bytes as f64) / (self.bandwidth_gbps * 1000.0)
    }
}

/// PCIe 4.0 x16 bandwidth in GB/s.
pub const PCIE4_X16_BANDWIDTH_GBPS: f64 = 31.5;

/// Estimated latency for PCIe DMA transfer in microseconds.
pub const PCIE_DMA_LATENCY_US: f64 = 5.0;

/// Empirical GPU dispatch overhead in microseconds (queue submit + readback).
pub const GPU_DISPATCH_OVERHEAD_US: f64 = 1500.0;

// =============================================================================
// BandwidthTier — PCIe generation classification for transfer cost modelling
// =============================================================================

/// PCIe/interconnect bandwidth tier for transfer cost estimation.
///
/// Runtime-detected from GPU adapter name heuristics. Used by
/// `dispatch_with_transfer_cost()` to factor data movement cost
/// into the CPU/GPU dispatch decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BandwidthTier {
    /// PCIe 3.0 x16 — ~15.75 GB/s (Titan V, RTX 2000 series)
    PciE3x16,
    /// PCIe 4.0 x16 — ~31.5 GB/s (RTX 3000/4000, RX 6000/7000)
    PciE4x16,
    /// PCIe 5.0 x16 — ~63 GB/s (next-gen data center)
    PciE5x16,
    /// NVLink — ~300 GB/s (A100, H100 multi-GPU)
    NvLink,
    /// Shared/unified memory — effectively infinite (Apple M-series, integrated)
    SharedMemory,
    /// Unknown — conservative fallback (PCIe 3.0 assumptions)
    Unknown,
}

impl BandwidthTier {
    /// Theoretical unidirectional bandwidth in GB/s.
    #[must_use]
    pub const fn bandwidth_gbps(self) -> f64 {
        match self {
            BandwidthTier::PciE3x16 => 15.75,
            BandwidthTier::PciE4x16 => 31.5,
            BandwidthTier::PciE5x16 => 63.0,
            BandwidthTier::NvLink => 300.0,
            BandwidthTier::SharedMemory => 1000.0,
            BandwidthTier::Unknown => 15.75,
        }
    }

    /// Estimated DMA latency in microseconds.
    #[must_use]
    pub const fn latency_us(self) -> f64 {
        match self {
            BandwidthTier::SharedMemory => 0.1,
            BandwidthTier::NvLink => 1.0,
            _ => PCIE_DMA_LATENCY_US,
        }
    }

    /// Build a `TransferCost` from this tier's characteristics.
    #[must_use]
    pub const fn transfer_cost(self) -> TransferCost {
        TransferCost {
            latency_us: self.latency_us(),
            bandwidth_gbps: self.bandwidth_gbps(),
        }
    }

    /// Detect bandwidth tier from GPU adapter name (heuristic).
    ///
    /// Falls back to `Unknown` when the name doesn't match any known pattern.
    #[must_use]
    pub fn detect_from_adapter_name(name: &str) -> Self {
        let lower = name.to_lowercase();

        // NVLink: data-center GPUs (multi-GPU interconnect)
        if lower.contains("a100")
            || lower.contains("h100")
            || lower.contains("h200")
        {
            return BandwidthTier::NvLink;
        }

        // Shared memory: Apple Silicon (unified), software renderers
        if lower.contains("apple")
            || lower.contains("llvmpipe")
            || lower.contains("swiftshader")
        {
            return BandwidthTier::SharedMemory;
        }

        // PCIe 5.0: next-gen data center
        if lower.contains("b100") || lower.contains("b200") {
            return BandwidthTier::PciE5x16;
        }

        // PCIe 4.0: RTX 3000/4000, RX 6000/7000, Intel Arc, MI200+
        if lower.contains("rtx 30")
            || lower.contains("rtx 40")
            || lower.contains("rx 6")
            || lower.contains("rx 7")
            || lower.contains("arc")
            || lower.contains("a770")
            || lower.contains("a750")
            || lower.contains("mi2")
            || lower.contains("mi3")
        {
            return BandwidthTier::PciE4x16;
        }

        // PCIe 3.0: RTX 2000, Titan V, V100
        if lower.contains("rtx 20")
            || lower.contains("titan v")
            || lower.contains("v100")
            || lower.contains("gv100")
        {
            return BandwidthTier::PciE3x16;
        }

        BandwidthTier::Unknown
    }
}

/// Select optimal mixed substrate for a workload.
///
/// Uses a cost model: if compute dominates transfer, route to target;
/// otherwise stay on source to avoid transfer overhead.
///
/// Uses default PCIe 4.0 bandwidth. For tier-aware routing, use
/// [`mixed_substrate_with_tier`].
#[must_use]
pub fn mixed_substrate(
    compute_us: f64,
    data_bytes: usize,
    source: HardwareType,
    target: HardwareType,
) -> MixedSubstrate {
    mixed_substrate_with_tier(compute_us, data_bytes, source, target, BandwidthTier::PciE4x16)
}

/// Select optimal mixed substrate with explicit bandwidth tier.
///
/// Like [`mixed_substrate`] but uses the provided [`BandwidthTier`] for
/// transfer cost estimation instead of defaulting to PCIe 4.0.
#[must_use]
pub fn mixed_substrate_with_tier(
    compute_us: f64,
    data_bytes: usize,
    source: HardwareType,
    target: HardwareType,
    tier: BandwidthTier,
) -> MixedSubstrate {
    if source == target {
        return match source {
            HardwareType::GPU => MixedSubstrate::GpuOnly,
            HardwareType::CPU => MixedSubstrate::CpuOnly,
            HardwareType::NPU => MixedSubstrate::NpuOnly,
            _ => MixedSubstrate::CpuOnly,
        };
    }

    let cost = tier.transfer_cost();
    let transfer_us = cost.estimated_us(data_bytes) + GPU_DISPATCH_OVERHEAD_US;

    if compute_us > transfer_us {
        match (source, target) {
            (HardwareType::GPU, HardwareType::CPU) => MixedSubstrate::GpuToCpu,
            (HardwareType::CPU, HardwareType::GPU) => MixedSubstrate::CpuToGpu,
            (HardwareType::GPU, HardwareType::NPU) => MixedSubstrate::GpuToNpu,
            (HardwareType::NPU, HardwareType::GPU) => MixedSubstrate::NpuToGpu,
            _ => match source {
                HardwareType::GPU => MixedSubstrate::GpuOnly,
                HardwareType::NPU => MixedSubstrate::NpuOnly,
                _ => MixedSubstrate::CpuOnly,
            },
        }
    } else {
        match source {
            HardwareType::GPU => MixedSubstrate::GpuOnly,
            HardwareType::CPU => MixedSubstrate::CpuOnly,
            HardwareType::NPU => MixedSubstrate::NpuOnly,
            _ => MixedSubstrate::CpuOnly,
        }
    }
}

/// PCIe bridge for GPU↔NPU transfer cost estimation.
///
/// Minimal implementation; real P2P detection is not yet available.
pub struct PcieBridge {
    pub p2p_available: bool,
    pub source_label: String,
    pub target_label: String,
}

impl PcieBridge {
    /// Detect P2P capability. Returns false (safe default) until real detection is implemented.
    #[must_use]
    pub fn detect_p2p() -> Self {
        Self {
            p2p_available: false,
            source_label: "unknown".to_string(),
            target_label: "unknown".to_string(),
        }
    }

    /// Estimate transfer cost for given byte count.
    #[must_use]
    pub fn transfer_cost(&self, _bytes: usize) -> TransferCost {
        TransferCost {
            latency_us: PCIE_DMA_LATENCY_US,
            bandwidth_gbps: PCIE4_X16_BANDWIDTH_GBPS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_type() {
        assert_eq!(HardwareType::CPU, HardwareType::CPU);
        assert_ne!(HardwareType::CPU, HardwareType::GPU);
    }

    #[tokio::test]
    async fn test_hardware_discovery() {
        let executors = HardwareDiscovery::discover_all().await.unwrap();
        // At least CPU should be available
        assert!(!executors.is_empty());
        assert!(executors
            .iter()
            .any(|e| e.hardware_type() == HardwareType::CPU));
    }

    #[test]
    fn test_cpu_executor() {
        let cpu = CpuExecutor::new();
        assert_eq!(cpu.name(), "CPU (Native)");
        assert_eq!(cpu.hardware_type(), HardwareType::CPU);
        assert!(cpu.capabilities().operations.matmul);
    }

    #[test]
    fn test_mixed_substrate_same_device() {
        assert_eq!(
            mixed_substrate(100.0, 1024, HardwareType::GPU, HardwareType::GPU),
            MixedSubstrate::GpuOnly
        );
        assert_eq!(
            mixed_substrate(100.0, 1024, HardwareType::CPU, HardwareType::CPU),
            MixedSubstrate::CpuOnly
        );
        assert_eq!(
            mixed_substrate(100.0, 1024, HardwareType::NPU, HardwareType::NPU),
            MixedSubstrate::NpuOnly
        );
    }

    #[test]
    fn test_mixed_substrate_small_compute_stays_on_source() {
        // Small compute: transfer overhead dominates, stay on CPU
        let sub = mixed_substrate(100.0, 1024, HardwareType::CPU, HardwareType::GPU);
        assert_eq!(sub, MixedSubstrate::CpuOnly);
    }

    #[test]
    fn test_mixed_substrate_large_compute_transfers_to_target() {
        // Large compute: worth transferring CPU -> GPU
        let sub = mixed_substrate(100_000.0, 1_048_576, HardwareType::CPU, HardwareType::GPU);
        assert_eq!(sub, MixedSubstrate::CpuToGpu);
    }

    #[test]
    fn test_mixed_substrate_gpu_to_cpu() {
        let sub = mixed_substrate(100_000.0, 1_048_576, HardwareType::GPU, HardwareType::CPU);
        assert_eq!(sub, MixedSubstrate::GpuToCpu);
    }

    #[test]
    fn test_pcie_bridge_detect_p2p_returns_false() {
        let bridge = PcieBridge::detect_p2p();
        assert!(!bridge.p2p_available);
        assert_eq!(bridge.source_label, "unknown");
        assert_eq!(bridge.target_label, "unknown");
    }

    #[test]
    fn test_pcie_bridge_transfer_cost() {
        let bridge = PcieBridge::detect_p2p();
        let cost = bridge.transfer_cost(1_048_576);
        assert!(cost.latency_us > 0.0);
        assert!(cost.bandwidth_gbps > 0.0);
        assert!(cost.estimated_us(1_048_576) > cost.latency_us);
    }

    // BandwidthTier tests

    #[test]
    fn test_bandwidth_tier_values() {
        assert!((BandwidthTier::PciE3x16.bandwidth_gbps() - 15.75).abs() < 0.01);
        assert!((BandwidthTier::PciE4x16.bandwidth_gbps() - 31.5).abs() < 0.01);
        assert!((BandwidthTier::PciE5x16.bandwidth_gbps() - 63.0).abs() < 0.01);
        assert!(BandwidthTier::NvLink.bandwidth_gbps() > 200.0);
        assert!(BandwidthTier::SharedMemory.bandwidth_gbps() > 500.0);
    }

    #[test]
    fn test_bandwidth_tier_detect_nvidia() {
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("NVIDIA GeForce RTX 4070"),
            BandwidthTier::PciE4x16
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("NVIDIA GeForce RTX 3090"),
            BandwidthTier::PciE4x16
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("NVIDIA GeForce RTX 2080 Ti"),
            BandwidthTier::PciE3x16
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("NVIDIA TITAN V"),
            BandwidthTier::PciE3x16
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("NVIDIA A100"),
            BandwidthTier::NvLink
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("NVIDIA H100"),
            BandwidthTier::NvLink
        );
    }

    #[test]
    fn test_bandwidth_tier_detect_amd() {
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("AMD Radeon RX 7900 XTX"),
            BandwidthTier::PciE4x16
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("AMD Radeon RX 6950 XT"),
            BandwidthTier::PciE4x16
        );
    }

    #[test]
    fn test_bandwidth_tier_detect_special() {
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("Apple M2 Pro"),
            BandwidthTier::SharedMemory
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("llvmpipe"),
            BandwidthTier::SharedMemory
        );
        assert_eq!(
            BandwidthTier::detect_from_adapter_name("Unknown GPU XYZ"),
            BandwidthTier::Unknown
        );
    }

    #[test]
    fn test_bandwidth_tier_transfer_cost() {
        let cost = BandwidthTier::PciE4x16.transfer_cost();
        assert!((cost.bandwidth_gbps - 31.5).abs() < 0.01);
        let one_mb_us = cost.estimated_us(1_048_576);
        assert!(one_mb_us > cost.latency_us);
    }

    #[test]
    fn test_mixed_substrate_with_tier_shared_memory_transfers() {
        // Compute time must exceed dispatch overhead (1500 µs) + negligible
        // transfer cost for SharedMemory to trigger a cross-device transfer.
        let sub = mixed_substrate_with_tier(
            5_000.0,
            1_048_576,
            HardwareType::CPU,
            HardwareType::GPU,
            BandwidthTier::SharedMemory,
        );
        assert_eq!(sub, MixedSubstrate::CpuToGpu);
    }

    #[test]
    fn test_mixed_substrate_with_tier_pcie3_avoids_small_transfer() {
        let sub = mixed_substrate_with_tier(
            100.0, // small compute
            1024,
            HardwareType::CPU,
            HardwareType::GPU,
            BandwidthTier::PciE3x16,
        );
        assert_eq!(sub, MixedSubstrate::CpuOnly);
    }
}
