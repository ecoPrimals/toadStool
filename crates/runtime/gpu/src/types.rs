// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type definitions for Universal GPU Compute Runtime

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

/// GPU compute frameworks supported by the runtime
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuFramework {
    /// Universal `WebGPU` (future-ready, cross-platform)
    WebGpu,
    /// Vulkan compute (cross-platform, high-performance)
    Vulkan,
    /// Serialization / discovery only: OpenCL-class GPUs use the `gpu.dispatch.opencl` capability provider via IPC.
    #[deprecated(
        since = "0.2.0",
        note = "OpenCL removed S198 — use gpu.dispatch.opencl capability provider via IPC"
    )]
    OpenCl,
    /// NVIDIA CUDA (NVIDIA-specific, high-performance)
    Cuda,
    /// Apple Metal (Apple-specific, optimized)
    Metal,
    /// AMD ROCm/HIP (AMD-specific, high-performance)
    Rocm,
    /// Microsoft `DirectCompute` (Windows-specific)
    DirectCompute,
    /// Custom/plugin framework
    Custom(String),
}

impl GpuFramework {
    /// Get human-readable framework name
    #[must_use]
    #[expect(deprecated, reason = "exhaustive match includes deprecated OpenCl variant")]
    pub fn name(&self) -> &str {
        match self {
            Self::WebGpu => "WebGPU",
            Self::Vulkan => "Vulkan",
            Self::OpenCl => "OpenCL",
            Self::Cuda => "CUDA",
            Self::Metal => "Metal",
            Self::Rocm => "ROCm",
            Self::DirectCompute => "DirectCompute",
            Self::Custom(name) => name,
        }
    }

    /// Check if framework is universally supported
    #[must_use]
    pub const fn is_universal(&self) -> bool {
        matches!(self, Self::WebGpu | Self::Vulkan)
    }

    /// Get platform compatibility information
    #[must_use]
    #[expect(deprecated, reason = "exhaustive match includes deprecated OpenCl variant")]
    pub fn platform_compatibility(&self) -> Vec<&str> {
        match self {
            Self::WebGpu => vec!["Windows", "macOS", "Linux", "Web"],
            Self::Vulkan => vec!["Windows", "macOS", "Linux", "Android"],
            Self::OpenCl => vec!["Windows", "macOS", "Linux"],
            Self::Cuda => vec!["Windows", "Linux"],
            Self::Metal => vec!["macOS", "iOS"],
            Self::Rocm => vec!["Linux"],
            Self::DirectCompute => vec!["Windows"],
            Self::Custom(_) => vec!["Unknown"],
        }
    }
}

/// Device identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId {
    /// Framework this device belongs to
    pub framework: GpuFramework,
    /// Framework-specific device ID
    pub device_index: u32,
    /// Unique identifier
    pub uuid: String,
}

impl DeviceId {
    /// Creates a new device ID.
    #[must_use]
    pub const fn new(framework: GpuFramework, device_index: u32, uuid: String) -> Self {
        Self {
            framework,
            device_index,
            uuid,
        }
    }
}

/// Universal compute device representation
#[derive(Debug)]
pub struct UniversalComputeDevice {
    /// Device identifier
    pub id: DeviceId,
    /// Device metadata
    pub info: DeviceInfo,
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
    /// Current resource usage
    pub usage: Arc<RwLock<DeviceUsage>>,
    /// Framework-specific handle
    pub framework_handle: Option<FrameworkHandle>,
}

impl Clone for UniversalComputeDevice {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            info: self.info.clone(),
            capabilities: self.capabilities.clone(),
            usage: Arc::clone(&self.usage),
            framework_handle: self.framework_handle.clone(),
        }
    }
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Human-readable device name
    pub name: String,
    /// Device vendor
    pub vendor: String,
    /// Device type
    pub device_type: DeviceType,
    /// Driver version
    pub driver_version: String,
    /// Hardware architecture
    pub architecture: String,
    /// Physical location (for multi-GPU systems)
    pub physical_location: Option<String>,
}

/// Device types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    /// Discrete GPU (dedicated graphics card)
    DiscreteGpu,
    /// Integrated GPU (CPU-integrated graphics)
    IntegratedGpu,
    /// APU (Accelerated Processing Unit)
    Apu,
    /// Compute-only device (no display output)
    ComputeOnly,
    /// Virtual GPU (cloud/virtualized)
    VirtualGpu,
    /// Other/unknown device type
    Other(String),
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Compute capability/version
    pub compute_capability: String,
    /// Total memory in bytes
    pub total_memory_bytes: u64,
    /// Memory bandwidth in GB/s
    pub memory_bandwidth_gbps: f64,
    /// Number of compute units/cores
    pub compute_units: u32,
    /// Maximum work group size
    pub max_work_group_size: (u32, u32, u32),
    /// Supported data types
    pub supported_data_types: Vec<DataType>,
    /// Supported extensions/features
    pub extensions: HashMap<String, bool>,
    /// Performance characteristics
    pub performance: PerformanceCharacteristics,
}

/// Performance characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCharacteristics {
    /// Peak GFLOPS (single precision)
    pub peak_gflops_fp32: f64,
    /// Peak GFLOPS (double precision)
    pub peak_gflops_fp64: Option<f64>,
    /// Peak GFLOPS (half precision)
    pub peak_gflops_fp16: Option<f64>,
    /// Peak memory bandwidth utilization
    pub peak_memory_bandwidth_utilization: f64,
    /// Typical power consumption in watts
    pub typical_power_watts: f64,
    /// Maximum power consumption in watts
    pub max_power_watts: f64,
}

/// Supported data types for GPU buffers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    /// 8-bit signed integer.
    Int8,
    /// 16-bit signed integer.
    Int16,
    /// 32-bit signed integer.
    Int32,
    /// 64-bit signed integer.
    Int64,
    /// 8-bit unsigned integer.
    UInt8,
    /// 16-bit unsigned integer.
    UInt16,
    /// 32-bit unsigned integer.
    UInt32,
    /// 64-bit unsigned integer.
    UInt64,
    /// 16-bit float.
    Float16,
    /// 32-bit float.
    Float32,
    /// 64-bit float.
    Float64,
    /// 64-bit complex.
    Complex64,
    /// 128-bit complex.
    Complex128,
    /// Boolean.
    Bool,
    /// Custom type.
    Custom(String),
}

/// Current device usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceUsage {
    /// GPU utilization percentage (0-100)
    pub gpu_utilization_percent: f32,
    /// Memory utilization percentage (0-100)
    pub memory_utilization_percent: f32,
    /// Current memory usage in bytes
    pub memory_used_bytes: u64,
    /// Current temperature in Celsius
    pub temperature_celsius: Option<f32>,
    /// Current power usage in watts
    pub power_usage_watts: Option<f32>,
    /// Number of active compute sessions
    pub active_sessions: u32,
}

impl Default for DeviceUsage {
    fn default() -> Self {
        Self {
            gpu_utilization_percent: 0.0,
            memory_utilization_percent: 0.0,
            memory_used_bytes: 0,
            temperature_celsius: None,
            power_usage_watts: None,
            active_sessions: 0,
        }
    }
}

/// Framework-specific device handles.
#[derive(Debug)]
pub enum FrameworkHandle {
    /// Vulkan device.
    #[cfg(feature = "vulkan")]
    Vulkan(Arc<vulkano::device::Device>),
    /// `WebGPU` device.
    #[cfg(feature = "webgpu")]
    WebGpu(Arc<wgpu::Device>),
    // #[cfg(feature = "metal")]
    // Metal(metal::Device), // Not available on Linux
    /// Framework was detected but could not provide a real device handle.
    Unavailable {
        /// Framework name.
        name: String,
        /// Reason for unavailability.
        reason: String,
    },
}

impl Clone for FrameworkHandle {
    fn clone(&self) -> Self {
        match self {
            #[cfg(feature = "vulkan")]
            Self::Vulkan(device) => Self::Vulkan(Arc::clone(device)),
            #[cfg(feature = "webgpu")]
            Self::WebGpu(device) => Self::WebGpu(Arc::clone(device)),
            Self::Unavailable { name, reason } => Self::Unavailable {
                name: name.clone(),
                reason: reason.clone(),
            },
        }
    }
}

/// Device requirements for workload execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRequirements {
    /// Minimum memory in bytes
    pub min_memory_bytes: Option<u64>,
    /// Minimum compute units
    pub min_compute_units: Option<u32>,
    /// Required data types
    pub required_data_types: Vec<DataType>,
    /// Required extensions
    pub required_extensions: Vec<String>,
    /// Preferred device types
    pub preferred_device_types: Vec<DeviceType>,
    /// Minimum compute capability
    pub min_compute_capability: Option<String>,
}

impl DeviceRequirements {
    /// Create minimal device requirements
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            min_memory_bytes: Some(64 * 1024 * 1024), // 64MB
            min_compute_units: Some(1),
            required_data_types: vec![DataType::Float32],
            required_extensions: vec![],
            preferred_device_types: vec![],
            min_compute_capability: None,
        }
    }

    /// Create high-performance device requirements
    #[must_use]
    pub fn high_performance() -> Self {
        Self {
            min_memory_bytes: Some(4 * 1024 * 1024 * 1024), // 4GB
            min_compute_units: Some(16),
            required_data_types: vec![DataType::Float32, DataType::Float64],
            required_extensions: vec![],
            preferred_device_types: vec![DeviceType::DiscreteGpu, DeviceType::ComputeOnly],
            min_compute_capability: Some("6.0".to_string()),
        }
    }
}

/// Compute session information
#[derive(Debug, Clone)]
pub struct ComputeSession {
    /// Session ID
    pub id: Uuid,
    /// Device being used
    pub device_id: DeviceId,
    /// Parent session (for recursive execution)
    pub parent_session: Option<Uuid>,
    /// Child sessions spawned by this session
    pub child_sessions: Vec<Uuid>,
    /// Recursion depth
    pub recursion_depth: u32,
    /// Session start time
    pub start_time: Instant,
    /// Resource allocation
    pub resource_allocation: ResourceAllocation,
    /// Current status
    pub status: SessionStatus,
}

/// Resource allocation for a compute session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Allocated memory in bytes
    pub memory_bytes: u64,
    /// Allocated compute units
    pub compute_units: u32,
    /// Priority level
    pub priority: u32,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is initializing
    Initializing,
    /// Session is running
    Running,
    /// Session is paused
    Paused,
    /// Session completed successfully
    Completed,
    /// Session failed
    Failed(String),
    /// Session was cancelled
    Cancelled,
}

/// Kernel formats supported by the compiler
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub enum KernelFormat {
    /// `OpenCL` C
    OpenClC,
    /// CUDA C
    CudaC,
    /// HLSL compute shaders
    Hlsl,
    /// GLSL compute shaders
    Glsl,
    /// Metal shading language
    Msl,
    /// SPIR-V binary
    Spirv,
    /// LLVM IR
    LlvmIr,
    /// WebAssembly
    Wasm,
    /// `WebGPU` Shading Language (WGSL)
    Wgsl,
    /// `ToadStool` Universal Compute Language (custom)
    Tucl,
}

/// Compiled kernel information
#[derive(Debug, Clone)]
pub struct CompiledKernel {
    /// Kernel ID
    pub id: String,
    /// Compiled binary/code (zero-copy via refcounted `Bytes`)
    pub binary: bytes::Bytes,
    /// Target framework
    pub framework: GpuFramework,
    /// Compilation timestamp
    pub compiled_at: Instant,
    /// Optimization level used
    pub optimization_level: super::config::OptimizationLevel,
    /// Resource requirements
    pub resource_requirements: ResourceAllocation,
}

/// Kernel input parameter
#[derive(Debug, Clone)]
pub struct KernelInput {
    /// Parameter name
    pub name: String,
    /// Input data (zero-copy via refcounted `Bytes`)
    pub data: bytes::Bytes,
    /// Data type
    pub data_type: DataType,
    /// Access pattern (read-only, write-only, read-write)
    pub access_pattern: AccessPattern,
}

/// Memory access patterns for buffers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessPattern {
    /// Read-only access.
    ReadOnly,
    /// Write-only access.
    WriteOnly,
    /// Read-write access.
    ReadWrite,
}

/// Kernel execution output
#[derive(Debug, Clone)]
pub struct KernelOutput {
    /// Output data buffers (zero-copy via refcounted `Bytes`)
    pub buffers: HashMap<String, bytes::Bytes>,
    /// Execution metrics
    pub metrics: ExecutionMetrics,
    /// Any error information
    pub errors: Vec<String>,
}

/// Execution metrics
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    /// Execution time
    pub execution_time: std::time::Duration,
    /// Memory used in bytes
    pub memory_used: u64,
    /// Compute units utilized
    pub compute_units_used: u32,
    /// Energy consumed in joules
    pub energy_consumed: Option<f64>,
    /// Throughput metrics
    pub throughput: Option<ThroughputMetrics>,
}

/// Throughput metrics
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    /// Operations per second
    pub ops_per_second: f64,
    /// Data processed per second (bytes)
    pub bytes_per_second: f64,
    /// Memory bandwidth utilization (percentage)
    pub memory_bandwidth_utilization: f64,
}

/// Compute workload specification
#[derive(Debug, Clone)]
pub struct ComputeWorkload {
    /// Workload name
    pub name: String,
    /// Kernel source code
    pub kernel_source: String,
    /// Kernel format
    pub kernel_format: KernelFormat,
    /// Input data
    pub inputs: Vec<KernelInput>,
    /// Device requirements
    pub requirements: DeviceRequirements,
    /// Parent session (for recursive execution)
    pub parent_session: Option<Uuid>,
    /// Recursive workloads to spawn
    pub recursive_workloads: Vec<Self>,
    /// Execution priority
    pub priority: u32,
}

/// Compute result
#[derive(Debug, Clone)]
pub struct ComputeResult {
    /// Session that executed this workload
    pub session_id: Uuid,
    /// Device used for execution
    pub device_id: DeviceId,
    /// Primary kernel output
    pub primary_output: KernelOutput,
    /// Results from recursive workloads
    pub recursive_results: Vec<Self>,
    /// Total execution time
    pub total_execution_time: std::time::Duration,
}

/// Compute engine statistics
#[derive(Debug, Clone)]
pub struct ComputeEngineStatistics {
    /// Total devices available
    pub total_devices: usize,
    /// Active compute sessions
    pub active_sessions: usize,
    /// Available frameworks
    pub frameworks_available: usize,
    /// Sessions with recursion
    pub recursive_sessions: usize,
    /// Maximum recursion depth currently active
    pub max_recursion_depth: u32,
}

/// Resource pool for device management
#[derive(Debug, Clone)]
pub struct ResourcePool {
    /// Total memory available
    pub total_memory: u64,
    /// Currently allocated memory
    pub allocated_memory: u64,
    /// Total compute units
    pub total_compute_units: u32,
    /// Currently allocated compute units
    pub allocated_compute_units: u32,
    /// Allocation queue
    pub allocation_queue: Vec<(Uuid, ResourceAllocation)>,
}
