// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execution traits and workload / kernel / buffer types.

use super::detection::*;
use super::policy::*;
use crate::types::DataType;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use toadstool::error::ToadStoolResult;
use uuid::Uuid;

/// Boxed Send future (avoids clippy::type_complexity on nested generics).
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Universal compute resource trait - GPU, CPU, TPU, anything!
// NOTE(async-dyn): `Pin<Box<dyn Future>>` for object-safe async methods on trait objects.
pub trait UniversalComputeResource: Send + Sync {
    /// Get capabilities of this resource
    fn capabilities(&self) -> &ComputeCapabilities;

    /// Get unique identifier for this resource
    fn resource_id(&self) -> &str;

    /// Check if can execute this workload
    fn can_execute(&self, requirements: &ComputeRequirements) -> bool {
        self.capabilities().meets_requirements(requirements)
    }

    /// Score how well this resource matches workload (0.0-1.0)
    fn score_workload(&self, requirements: &ComputeRequirements) -> f64 {
        if !self.can_execute(requirements) {
            return 0.0;
        }
        self.capabilities().score_for_workload(requirements)
    }

    /// Create execution context for this resource
    fn create_context(&self) -> BoxFuture<'_, ToadStoolResult<Box<dyn ComputeContext>>>;

    /// Get current utilization (0.0 = idle, 1.0 = fully utilized)
    fn utilization(&self) -> Pin<Box<dyn Future<Output = f32> + Send + '_>>;

    /// Estimate execution time for workload
    fn estimate_execution_time(&self, requirements: &ComputeRequirements) -> Duration;
}

/// Compute execution context (session on a specific resource)
// NOTE(async-dyn): `Pin<Box<dyn Future>>` for object-safe async methods on trait objects.
pub trait ComputeContext: Send + Sync {
    /// Get context ID
    fn context_id(&self) -> Uuid;

    /// Get resource this context is on
    fn resource_id(&self) -> &str;

    /// Execute workload in this context
    fn execute<'a>(
        &'a mut self,
        workload: &'a UniversalWorkload,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<WorkloadResult>> + Send + 'a>>;

    /// Close context and cleanup resources
    fn close(self: Box<Self>) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send>>;
}

/// Universal workload description (hardware-agnostic)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UniversalWorkload {
    /// Unique workload ID
    pub id: String,

    /// What capabilities does this workload need?
    pub requirements: ComputeRequirements,

    /// The compute kernel
    pub kernel: UniversalKernel,

    /// Input buffers
    pub inputs: Vec<ComputeBuffer>,

    /// Expected output size
    pub output_size: usize,

    /// Optimization hints
    pub hints: OptimizationHints,
}

/// Buffer for compute data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComputeBuffer {
    /// Buffer name.
    pub name: String,
    /// Buffer data.
    pub data: bytes::Bytes,
    /// Element data type.
    pub element_type: DataType,
}

/// Universal kernel representation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum UniversalKernel {
    /// Source code in a universal language.
    Source {
        /// Kernel language.
        language: KernelLanguage,
        /// Source code.
        code: String,
        /// Entry point name.
        entry_point: String,
    },

    /// Pre-compiled binary.
    Binary {
        /// Binary format.
        format: BinaryFormat,
        /// Binary data.
        data: bytes::Bytes,
    },

    /// High-level operation description.
    Operation {
        /// Operation type.
        operation: Operation,
        /// Operation parameters.
        parameters: HashMap<String, serde_json::Value>,
    },

    /// Reference to standard library function.
    Library {
        /// Function name.
        name: String,
        /// Version string.
        version: String,
    },
}

/// Supported kernel languages.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KernelLanguage {
    /// WGSL (`WebGPU` Shading Language).
    Wgsl,
    /// SPIR-V.
    Spirv,
    /// `OpenCL` C (dispatch via barraCuda/coralReef; not compiled in-tree — S198).
    OpenClC,
    /// CUDA.
    Cuda,
    /// Metal Shading Language.
    Metal,
    /// Python (for high-level kernels).
    Python,
    /// Rust (for GPU kernels).
    Rust,
    /// Custom language.
    Custom(String),
}

/// Binary formats for pre-compiled kernels.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BinaryFormat {
    /// SPIR-V binary.
    SpirvBinary,
    /// Native binary.
    NativeBinary,
    /// Custom format.
    Custom(String),
}

/// Optimization hints for scheduler.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OptimizationHints {
    /// Prefer latency over throughput.
    pub low_latency: bool,

    /// Prefer energy efficiency.
    pub energy_efficient: bool,

    /// Allow approximate results
    pub approximate: bool,

    /// Priority (0 = low, 10 = critical)
    pub priority: u8,
}

/// Result of workload execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkloadResult {
    /// Output buffers (zero-copy via refcounted `Bytes`)
    pub outputs: HashMap<String, bytes::Bytes>,

    /// Execution metrics
    pub metrics: ExecutionMetrics,

    /// Errors or warnings
    pub messages: Vec<String>,
}

/// Execution metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionMetrics {
    /// Actual execution time
    pub execution_time: Duration,

    /// Memory used
    pub memory_used: u64,

    /// Energy consumed (if available)
    pub energy_joules: Option<f64>,

    /// Resource utilization during execution
    pub utilization: f32,
}
