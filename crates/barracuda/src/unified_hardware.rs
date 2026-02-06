//! Unified Hardware Base - Universal Compute Abstraction
//!
//! **Philosophy**: Hardware executes math, math doesn't know hardware
//!
//! This module provides the hardware abstraction layer for BarraCUDA:
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

use crate::error::Result;
use crate::unified_math::{MathOp, TensorDescriptor};
use async_trait::async_trait;
use std::sync::Arc;

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
    async fn transfer(
        &self,
        tensor: Arc<dyn TensorStorage>,
    ) -> Result<Arc<dyn TensorStorage>>;
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
                score_a.partial_cmp(&score_b).unwrap()
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
        let executor = self
            .select_executor(op, &descriptors)
            .ok_or_else(|| crate::error::BarracudaError::NoAvailableExecutor {
                operation: format!("{:?}", op),
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
        // TODO: Implement GPU discovery via wgpu
        Ok(Vec::new())
    }
    
    #[cfg(feature = "tpu")]
    async fn discover_tpus() -> Result<Vec<Arc<dyn ComputeExecutor>>> {
        // TODO: Implement TPU discovery
        Ok(Vec::new())
    }
    
    async fn discover_npus() -> Result<Vec<Arc<dyn ComputeExecutor>>> {
        // TODO: Implement NPU discovery
        Ok(Vec::new())
    }
}

/// CPU executor implementation (always available)
struct CpuExecutor {
    capabilities: HardwareCapabilities,
}

impl CpuExecutor {
    fn new() -> Self {
        Self {
            capabilities: HardwareCapabilities {
                hardware_type: HardwareType::CPU,
                parallelism: ParallelismCapabilities {
                    max_parallel_units: num_cpus::get(),
                    simd_width: 8, // AVX2
                    task_parallel: true,
                    data_parallel: true,
                    pipeline_parallel: true,
                },
                memory: MemoryCapabilities {
                    total_bytes: 16 * 1024 * 1024 * 1024, // Estimate 16GB
                    available_bytes: 8 * 1024 * 1024 * 1024,
                    bandwidth_bytes_per_sec: 50 * 1024 * 1024 * 1024, // 50 GB/s
                    unified_memory: true,
                    zero_copy: true,
                },
                precision: PrecisionCapabilities {
                    fp16: false,
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
                    peak_tflops_fp32: 0.5,
                    peak_tflops_fp16: 0.0,
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
        // CPU can execute everything (fallback)
        true
    }
    
    fn score_operation(&self, _op: &MathOp, _inputs: &[TensorDescriptor]) -> f64 {
        // CPU baseline score: 0.5
        // Other hardware should score higher for parallel ops
        0.5
    }
    
    async fn execute(
        &self,
        _op: &MathOp,
        _inputs: Vec<Arc<dyn TensorStorage>>,
    ) -> Result<Arc<dyn TensorStorage>> {
        // TODO: Implement CPU execution
        unimplemented!("CPU executor not yet implemented")
    }
    
    async fn allocate(&self, _descriptor: TensorDescriptor) -> Result<Arc<dyn TensorStorage>> {
        // TODO: Implement CPU allocation
        unimplemented!("CPU allocate not yet implemented")
    }
    
    async fn transfer(
        &self,
        tensor: Arc<dyn TensorStorage>,
    ) -> Result<Arc<dyn TensorStorage>> {
        // If already on CPU, return as-is
        if tensor.is_cpu() {
            Ok(tensor)
        } else {
            // TODO: Transfer from other device to CPU
            unimplemented!("Transfer to CPU not yet implemented")
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
        assert!(executors.iter().any(|e| e.hardware_type() == HardwareType::CPU));
    }

    #[test]
    fn test_cpu_executor() {
        let cpu = CpuExecutor::new();
        assert_eq!(cpu.name(), "CPU (Native)");
        assert_eq!(cpu.hardware_type(), HardwareType::CPU);
        assert!(cpu.capabilities().operations.matmul);
    }
}
