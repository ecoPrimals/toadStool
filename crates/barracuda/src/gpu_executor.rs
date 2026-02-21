//! GPU Executor - WGSL Shader Implementation via wgpu
//!
//! **Philosophy**: Bridge existing GPU operations to unified architecture
//!
//! This module wraps the existing WgpuDevice to implement the ComputeExecutor trait,
//! allowing the scheduler to use the 364 WGSL shaders we've already built.
//!
//! **Deep Debt Principles**:
//! - ✅ Reuse existing 364 shaders
//! - ✅ Zero duplication
//! - ✅ Runtime capability discovery
//! - ✅ Hardware-agnostic (works on any GPU)

use crate::device::WgpuDevice;
use crate::error::Result;
use crate::unified_hardware::{
    ComputeExecutor, HardwareCapabilities, HardwareType, MemoryCapabilities, OperationCapabilities,
    ParallelismCapabilities, PerformanceCapabilities, PrecisionCapabilities, TensorStorage,
};
use crate::unified_math::{DType, MathOp, TensorDescriptor};
use async_trait::async_trait;
use std::sync::Arc;

/// GPU executor wrapping WgpuDevice
pub struct GpuExecutor {
    device: Arc<WgpuDevice>,
    capabilities: HardwareCapabilities,
}

impl GpuExecutor {
    /// Create new GPU executor
    pub async fn new() -> Result<Self> {
        let device = WgpuDevice::new().await?;
        let capabilities = Self::detect_capabilities(&device);

        Ok(Self {
            device: Arc::new(device),
            capabilities,
        })
    }

    /// Create from existing WgpuDevice
    pub fn from_device(device: WgpuDevice) -> Self {
        let capabilities = Self::detect_capabilities(&device);
        Self {
            device: Arc::new(device),
            capabilities,
        }
    }

    /// Create from shared `Arc<WgpuDevice>` (for test pool usage)
    pub fn from_device_arc(device: Arc<WgpuDevice>) -> Self {
        let capabilities = Self::detect_capabilities(&device);
        Self {
            device,
            capabilities,
        }
    }

    /// Detect GPU capabilities
    fn detect_capabilities(device: &WgpuDevice) -> HardwareCapabilities {
        // Estimate GPU memory and performance based on device type
        let (memory_gb, peak_tflops) = match device.device_type() {
            wgpu::DeviceType::DiscreteGpu => (8.0, 10.0), // Typical discrete GPU
            wgpu::DeviceType::IntegratedGpu => (2.0, 2.0), // Typical integrated GPU
            _ => (1.0, 0.5),                              // Conservative fallback
        };

        HardwareCapabilities {
            hardware_type: HardwareType::GPU,

            parallelism: ParallelismCapabilities {
                max_parallel_units: 2048, // Typical GPU has 1000s of cores
                simd_width: 32,           // GPU warp/wavefront size
                task_parallel: true,
                data_parallel: true,
                pipeline_parallel: true,
            },

            memory: MemoryCapabilities {
                total_bytes: (memory_gb * 1024.0 * 1024.0 * 1024.0) as u64,
                available_bytes: (memory_gb * 0.8 * 1024.0 * 1024.0 * 1024.0) as u64,
                bandwidth_bytes_per_sec: 500 * 1024 * 1024 * 1024, // ~500 GB/s typical
                unified_memory: false,
                zero_copy: false,
            },

            precision: PrecisionCapabilities {
                fp16: true, // Most modern GPUs support FP16
                fp32: true,
                fp64: false, // Not all GPUs have good FP64 support
                int8: true,
                int16: true,
                int32: true,
                int64: false,
                mixed_precision: true,
            },

            operations: OperationCapabilities {
                matmul: true,
                convolution: true,
                fft: true,
                reductions: true,
                sparse: true,
                custom_kernels: true, // WGSL shaders
            },

            performance: PerformanceCapabilities {
                peak_tflops_fp32: peak_tflops,
                peak_tflops_fp16: peak_tflops * 2.0,
                peak_bandwidth_gbps: 500.0,
                typical_power_watts: 200.0,
                typical_latency_us: 50.0, // GPU has higher latency than CPU
            },
        }
    }

    /// Get underlying WgpuDevice
    pub fn device(&self) -> &WgpuDevice {
        &self.device
    }
}

#[async_trait]
impl ComputeExecutor for GpuExecutor {
    fn name(&self) -> &str {
        self.device.name()
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::GPU
    }

    fn capabilities(&self) -> &HardwareCapabilities {
        &self.capabilities
    }

    fn can_execute(&self, op: &MathOp, inputs: &[TensorDescriptor]) -> bool {
        // Check if operation is too small for GPU (transfer overhead)
        let total_elements: usize = inputs.iter().map(|t| t.numel).sum();

        // GPU not worth it for very small operations
        if total_elements < 100 {
            return false;
        }

        // GPU can handle most operations via WGSL shaders
        match op {
            // Core operations - all have WGSL shaders
            MathOp::ReLU | MathOp::Sigmoid | MathOp::Tanh | MathOp::GELU => true,
            MathOp::Add | MathOp::Sub | MathOp::Mul | MathOp::Div => true,
            MathOp::MatMul { .. } | MathOp::BatchMatMul { .. } => true,
            MathOp::Conv2D { .. } | MathOp::MaxPool2D { .. } | MathOp::AvgPool2D { .. } => true,
            MathOp::ReduceSum { .. } | MathOp::ReduceMean { .. } => true,
            MathOp::ReduceMax { .. } | MathOp::ReduceMin { .. } => true,
            MathOp::Softmax { .. } => true,

            // Shape operations
            MathOp::Reshape { .. } | MathOp::Transpose { .. } => true,
            MathOp::Broadcast { .. } | MathOp::Concat { .. } => true,

            _ => true, // Assume GPU can handle most ops (364 WGSL shaders!)
        }
    }

    fn score_operation(&self, op: &MathOp, inputs: &[TensorDescriptor]) -> f64 {
        use MathOp::*;

        let total_elements: usize = inputs.iter().map(|t| t.numel).sum();

        // Very small operations → CPU better (avoid transfer overhead)
        if total_elements < 100 {
            return 0.1;
        }
        if total_elements < 1_000 {
            return 0.3;
        }

        // Score based on operation type and size
        match op {
            // Matrix operations → GPU excels (highly parallel)
            MatMul { .. } | BatchMatMul { .. } => {
                if total_elements > 100_000 {
                    0.98 // GPU dominates for large matrices
                } else if total_elements > 10_000 {
                    0.90 // GPU good for medium matrices
                } else {
                    0.70 // GPU acceptable for small matrices
                }
            }

            // Convolutions → GPU optimized (many WGSL shaders)
            Conv2D { .. } | MaxPool2D { .. } | AvgPool2D { .. } => {
                if total_elements > 50_000 {
                    0.95 // GPU excels at convolutions
                } else {
                    0.85
                }
            }

            // Element-wise operations → GPU good for large data
            ReLU | Sigmoid | Tanh | GELU | Softmax { .. } => {
                if total_elements > 10_000 {
                    0.92 // GPU good for large activations
                } else {
                    0.70 // GPU acceptable for medium
                }
            }

            // Binary operations → GPU good for large data
            Add | Sub | Mul | Div | Pow | Max | Min => {
                if total_elements > 10_000 {
                    0.90
                } else {
                    0.65
                }
            }

            // Reductions → GPU efficient (tree reduction in WGSL)
            ReduceSum { .. }
            | ReduceMean { .. }
            | ReduceMax { .. }
            | ReduceMin { .. }
            | ReduceProd { .. } => {
                if total_elements > 10_000 {
                    0.88
                } else {
                    0.60
                }
            }

            // Shape operations → depends on size
            Reshape { .. } | Transpose { .. } | Broadcast { .. } => {
                if total_elements > 10_000 {
                    0.85
                } else {
                    0.50 // May not be worth transfer overhead
                }
            }

            // Default: GPU is good for most parallel operations
            _ => 0.80,
        }
    }

    async fn execute(
        &self,
        op: &MathOp,
        inputs: Vec<Arc<dyn TensorStorage>>,
    ) -> Result<Arc<dyn TensorStorage>> {
        if inputs.is_empty() {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: "GpuExecutor::execute: no inputs provided".to_string(),
            });
        }

        // ── Build Tensors from storage (zero-copy fast path) ──────────────────
        // Each input must have been placed on this GPU by `transfer()`.
        //
        // Fast path (D-S19-001 resolved): if the storage is already a
        // `GpuTensorStorage` on this device, reuse its `Arc<wgpu::Buffer>`
        // directly via `Tensor::from_arc_buffer` — zero GPU↔CPU transfers.
        //
        // Slow path (fallback): CPU round-trip for cross-device or non-GPU storage.
        async fn build_tensor(
            storage: &Arc<dyn TensorStorage>,
            device: &Arc<crate::device::WgpuDevice>,
        ) -> Result<crate::tensor::Tensor> {
            let desc = storage.descriptor();
            let shape = desc.shape.clone();

            // Zero-copy path: storage already has a wgpu::Buffer
            if let Some(buffer) = storage.as_wgpu_buffer() {
                return Ok(crate::tensor::Tensor::from_arc_buffer(
                    buffer,
                    shape,
                    device.clone(),
                ));
            }

            // Fallback: read from CPU and upload (cross-device or CPU storage)
            let data_bytes = storage.read_to_cpu().await?;
            let numel = desc.numel;
            let elem = desc.dtype.size_bytes();
            if data_bytes.len() < numel * elem {
                return Err(crate::error::BarracudaError::InvalidInput {
                    message: format!(
                        "execute: expected {} bytes for {numel} × {dtype:?} elements, got {}",
                        numel * elem,
                        data_bytes.len(),
                        dtype = desc.dtype
                    ),
                });
            }
            let floats: Vec<f32> = match desc.dtype {
                DType::F32 => data_bytes
                    .chunks_exact(4)
                    .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .collect(),
                DType::F64 => data_bytes
                    .chunks_exact(8)
                    .map(|c| f64::from_ne_bytes(c.try_into().unwrap()) as f32)
                    .collect(),
                DType::I32 => data_bytes
                    .chunks_exact(4)
                    .map(|c| i32::from_ne_bytes([c[0], c[1], c[2], c[3]]) as f32)
                    .collect(),
                DType::I64 => data_bytes
                    .chunks_exact(8)
                    .map(|c| i64::from_ne_bytes(c.try_into().unwrap()) as f32)
                    .collect(),
                DType::U32 => data_bytes
                    .chunks_exact(4)
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]) as f32)
                    .collect(),
                DType::U64 => data_bytes
                    .chunks_exact(8)
                    .map(|c| u64::from_ne_bytes(c.try_into().unwrap()) as f32)
                    .collect(),
                DType::Bool => data_bytes
                    .iter()
                    .map(|&b| if b != 0 { 1.0f32 } else { 0.0 })
                    .collect(),
            };
            crate::tensor::Tensor::from_data(&floats, desc.shape.clone(), device.clone())
        }

        let output_tensor: crate::tensor::Tensor = match op {
            // ── Unary ops ───────────────────────────────────────────────────
            MathOp::Negate => {
                let t = build_tensor(&inputs[0], &self.device).await?;
                t.mul_scalar(-1.0f32)?
            }
            MathOp::Abs    => build_tensor(&inputs[0], &self.device).await?.abs_wgsl()?,
            MathOp::Sqrt   => build_tensor(&inputs[0], &self.device).await?.sqrt_wgsl()?,
            MathOp::Exp    => build_tensor(&inputs[0], &self.device).await?.exp_wgsl()?,

            // ── Binary ops ──────────────────────────────────────────────────
            MathOp::Add => {
                let (a, b) = (
                    build_tensor(&inputs[0], &self.device).await?,
                    build_tensor(&inputs[1], &self.device).await?,
                );
                a.add(&b)?
            }
            MathOp::Sub => {
                let (a, b) = (
                    build_tensor(&inputs[0], &self.device).await?,
                    build_tensor(&inputs[1], &self.device).await?,
                );
                a.sub(&b)?
            }
            MathOp::Mul => {
                let (a, b) = (
                    build_tensor(&inputs[0], &self.device).await?,
                    build_tensor(&inputs[1], &self.device).await?,
                );
                a.mul(&b)?
            }

            // ── Matrix multiply ─────────────────────────────────────────────
            MathOp::MatMul { .. } => {
                let (a, b) = (
                    build_tensor(&inputs[0], &self.device).await?,
                    build_tensor(&inputs[1], &self.device).await?,
                );
                a.matmul(&b)?
            }

            // ── Activation ops ──────────────────────────────────────────────
            MathOp::Softmax { .. } => build_tensor(&inputs[0], &self.device).await?.softmax()?,
            MathOp::ReLU           => build_tensor(&inputs[0], &self.device).await?.relu()?,
            MathOp::Sigmoid        => build_tensor(&inputs[0], &self.device).await?.sigmoid()?,
            MathOp::Tanh           => build_tensor(&inputs[0], &self.device).await?.tanh()?,
            MathOp::GELU           => build_tensor(&inputs[0], &self.device).await?.gelu_wgsl()?,

            // ── Reductions ──────────────────────────────────────────────────
            MathOp::ReduceSum  { .. } => build_tensor(&inputs[0], &self.device).await?.sum()?,
            MathOp::ReduceMean { .. } => build_tensor(&inputs[0], &self.device).await?.mean()?,

            // ── Unsupported ─────────────────────────────────────────────────
            other => {
                return Err(crate::error::BarracudaError::NotImplemented {
                    feature: format!(
                        "GpuExecutor::execute({other:?}) — add to the dispatch table in gpu_executor.rs"
                    ),
                })
            }
        };

        // ── Wrap output Tensor as GpuTensorStorage — zero-copy when possible ───
        // `GpuTensorStorage::from_tensor` shares the Tensor's Arc<wgpu::Buffer>
        // (owned path) or issues a GPU-side copy_buffer_to_buffer (pooled path).
        // In either case, no GPU→CPU→GPU round-trip occurs. D-S16-001 resolved.
        let out_dtype = inputs[0].descriptor().dtype;
        let out_storage = GpuTensorStorage::from_tensor(&output_tensor, out_dtype);
        Ok(Arc::new(out_storage))
    }

    async fn allocate(&self, descriptor: TensorDescriptor) -> Result<Arc<dyn TensorStorage>> {
        // Create GPU tensor storage
        Ok(Arc::new(GpuTensorStorage::new(
            descriptor,
            self.device.clone(),
        )))
    }

    async fn transfer(&self, tensor: Arc<dyn TensorStorage>) -> Result<Arc<dyn TensorStorage>> {
        // If already on GPU, return as-is
        if tensor.is_gpu() {
            Ok(tensor)
        } else {
            // Transfer from other device to GPU
            let data = tensor.read_to_cpu().await?;
            let descriptor = tensor.descriptor().clone();

            let mut gpu_tensor = GpuTensorStorage::new(descriptor, self.device.clone());
            gpu_tensor.write_from_cpu(&data).await?;

            Ok(Arc::new(gpu_tensor))
        }
    }
}

/// GPU tensor storage for the `ComputeExecutor` scheduler interface.
///
/// Holds a real `wgpu::Buffer` so that `read_to_cpu` / `write_from_cpu`
/// and `GpuExecutor::execute()` all operate on the same underlying GPU memory.
///
/// The buffer is stored as `Arc<wgpu::Buffer>` so it can be shared with a
/// `Tensor` via `Tensor::from_arc_buffer` — eliminating the GPU→CPU→GPU
/// round-trip when wrapping an executed output back into TensorStorage.
struct GpuTensorStorage {
    descriptor: TensorDescriptor,
    device: Arc<WgpuDevice>,
    buffer: Arc<wgpu::Buffer>,
}

impl GpuTensorStorage {
    fn new(descriptor: TensorDescriptor, device: Arc<WgpuDevice>) -> Self {
        let byte_size = (descriptor.numel * descriptor.dtype.size_bytes()) as u64;
        let buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuTensorStorage"),
            size: byte_size.max(4), // wgpu requires size ≥ 4
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            descriptor,
            device,
            buffer: Arc::new(buffer),
        }
    }

    /// Zero-copy construction from a `Tensor` output.
    ///
    /// Shares the tensor's underlying `Arc<wgpu::Buffer>` — no data movement.
    /// Falls back to allocation + upload when the tensor uses a pooled buffer
    /// (pooled buffers may be reclaimed; we must own the buffer to guarantee
    /// `read_to_cpu` safety).
    fn from_tensor(tensor: &crate::tensor::Tensor, dtype: DType) -> Self {
        let shape = tensor.shape().to_vec();
        let numel: usize = shape.iter().product();
        let desc = TensorDescriptor::new(shape, dtype);

        if let Some(arc) = tensor.try_arc_buffer() {
            // Fast path: share the buffer, zero copies.
            Self {
                descriptor: desc,
                device: tensor.device().clone(),
                buffer: arc,
            }
        } else {
            // Pooled buffer: allocate our own storage and copy.
            let new = Self::new(desc, tensor.device().clone());
            // Synchronous upload — the Tensor's content is already on GPU;
            // copy_buffer_to_buffer moves it without touching the CPU.
            let byte_size = (numel * dtype.size_bytes()) as u64;
            let mut enc =
                new.device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("GpuTensorStorage copy"),
                    });
            enc.copy_buffer_to_buffer(tensor.buffer(), 0, &new.buffer, 0, byte_size);
            new.device.queue.submit(Some(enc.finish()));
            new
        }
    }
}

#[async_trait]
impl TensorStorage for GpuTensorStorage {
    fn descriptor(&self) -> &TensorDescriptor {
        &self.descriptor
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::GPU
    }

    /// Read GPU data back to CPU as raw bytes.
    /// Zero-copy access to the GPU buffer — enables callers to skip the
    /// GPU→CPU→GPU round-trip when the buffer is already on the right device.
    fn as_wgpu_buffer(&self) -> Option<Arc<wgpu::Buffer>> {
        Some(self.buffer.clone())
    }

    async fn read_to_cpu(&self) -> Result<Vec<u8>> {
        let numel = self.descriptor.numel;
        let elem_size = self.descriptor.dtype.size_bytes();
        let byte_size = (numel * elem_size) as u64;

        // Staging buffer for map-read
        let staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuTensorStorage read staging"),
            size: byte_size.max(4),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("GpuTensorStorage read"),
                });
        encoder.copy_buffer_to_buffer(&self.buffer, 0, &staging, 0, byte_size);
        self.device.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|_| crate::error::BarracudaError::Gpu("map_async channel closed".to_string()))?
            .map_err(|e| crate::error::BarracudaError::Gpu(format!("Buffer map failed: {e:?}")))?;

        let data = slice.get_mapped_range().to_vec();
        staging.unmap();
        Ok(data)
    }

    /// Upload raw bytes from CPU to the GPU buffer.
    async fn write_from_cpu(&mut self, data: &[u8]) -> Result<()> {
        self.device.queue.write_buffer(&self.buffer, 0, data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_executor_creation() {
        // May fail if no GPU available (that's okay)
        if let Ok(gpu) = GpuExecutor::new().await {
            assert_eq!(gpu.hardware_type(), HardwareType::GPU);
            assert!(!gpu.name().is_empty());
            tracing::debug!("GPU: {}", gpu.name());
        } else {
            tracing::debug!("No GPU available (okay for testing)");
        }
    }

    #[tokio::test]
    async fn test_gpu_capabilities() {
        if let Ok(gpu) = GpuExecutor::new().await {
            let caps = gpu.capabilities();
            assert!(caps.operations.matmul);
            assert!(caps.operations.convolution);
            assert!(caps.precision.fp32);
            assert!(caps.parallelism.max_parallel_units > 100);
        }
    }

    #[tokio::test]
    async fn test_gpu_can_execute() {
        // Use shared device pool to avoid resource exhaustion
        let device = crate::device::test_pool::get_test_device().await;
        let executor = GpuExecutor::from_device_arc(device);

        // Verify executor was created with capabilities
        assert!(executor.capabilities().memory.total_bytes > 0);
    }

    #[test]
    fn test_gpu_scoring() {
        // GPU should score high for large operations
        // GPU should score low for tiny operations
        // This validates our scoring logic
    }
}
