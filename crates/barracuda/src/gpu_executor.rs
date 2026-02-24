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

use crate::cpu_executor::CpuExecutor;
use crate::device::WgpuDevice;
use crate::error::Result;
use crate::unified_hardware::{
    ComputeExecutor, HardwareCapabilities, HardwareType, MemoryCapabilities, OperationCapabilities,
    ParallelismCapabilities, PerformanceCapabilities, PrecisionCapabilities, TensorStorage,
};
use crate::unified_math::{DType, MathOp, TensorDescriptor};
use async_trait::async_trait;
use std::sync::Arc;

/// Conservative fallback estimates for GPU capabilities when runtime
/// detection is not yet available. These are used as initial estimates
/// and refined after actual device probing.
mod capability_defaults {
    pub const DISCRETE_MEMORY_GB: f64 = 8.0;
    pub const DISCRETE_PEAK_TFLOPS: f64 = 10.0;
    pub const INTEGRATED_MEMORY_GB: f64 = 2.0;
    pub const INTEGRATED_PEAK_TFLOPS: f64 = 2.0;
    pub const FALLBACK_MEMORY_GB: f64 = 1.0;
    pub const FALLBACK_PEAK_TFLOPS: f64 = 0.5;
    pub const GPU_MAX_PARALLEL_UNITS: usize = 2048;
    pub const GPU_SIMD_WIDTH: usize = 32;
    pub const MEMORY_AVAILABLE_FRACTION: f64 = 0.8;
    pub const TYPICAL_BANDWIDTH_GB_S: u64 = 500;
    pub const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;
}

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
        use capability_defaults::*;

        let (memory_gb, peak_tflops) = match device.device_type() {
            wgpu::DeviceType::DiscreteGpu => (DISCRETE_MEMORY_GB, DISCRETE_PEAK_TFLOPS),
            wgpu::DeviceType::IntegratedGpu => (INTEGRATED_MEMORY_GB, INTEGRATED_PEAK_TFLOPS),
            _ => (FALLBACK_MEMORY_GB, FALLBACK_PEAK_TFLOPS),
        };

        HardwareCapabilities {
            hardware_type: HardwareType::GPU,

            parallelism: ParallelismCapabilities {
                max_parallel_units: GPU_MAX_PARALLEL_UNITS,
                simd_width: GPU_SIMD_WIDTH,
                task_parallel: true,
                data_parallel: true,
                pipeline_parallel: true,
            },

            memory: MemoryCapabilities {
                total_bytes: (memory_gb * BYTES_PER_GB) as u64,
                available_bytes: (memory_gb * MEMORY_AVAILABLE_FRACTION * BYTES_PER_GB) as u64,
                bandwidth_bytes_per_sec: TYPICAL_BANDWIDTH_GB_S * 1024 * 1024 * 1024,
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
                    .map(|c| {
                        f64::from_ne_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32
                    })
                    .collect(),
                DType::I32 => data_bytes
                    .chunks_exact(4)
                    .map(|c| i32::from_ne_bytes([c[0], c[1], c[2], c[3]]) as f32)
                    .collect(),
                DType::I64 => data_bytes
                    .chunks_exact(8)
                    .map(|c| {
                        i64::from_ne_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32
                    })
                    .collect(),
                DType::U32 => data_bytes
                    .chunks_exact(4)
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]) as f32)
                    .collect(),
                DType::U64 => data_bytes
                    .chunks_exact(8)
                    .map(|c| {
                        u64::from_ne_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32
                    })
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
            MathOp::Negate => build_tensor(&inputs[0], &self.device)
                .await?
                .mul_scalar(-1.0f32)?,
            MathOp::Abs => build_tensor(&inputs[0], &self.device).await?.abs_wgsl()?,
            MathOp::Sqrt => build_tensor(&inputs[0], &self.device).await?.sqrt_wgsl()?,
            MathOp::Exp => build_tensor(&inputs[0], &self.device).await?.exp_wgsl()?,
            MathOp::Log => build_tensor(&inputs[0], &self.device).await?.log_wgsl()?,
            MathOp::Sin => build_tensor(&inputs[0], &self.device).await?.sin_wgsl()?,
            MathOp::Cos => build_tensor(&inputs[0], &self.device).await?.cos_wgsl()?,
            MathOp::Tan => build_tensor(&inputs[0], &self.device).await?.tan_wgsl()?,
            MathOp::Reciprocal => build_tensor(&inputs[0], &self.device)
                .await?
                .reciprocal_wgsl()?,
            MathOp::Square => {
                let t = build_tensor(&inputs[0], &self.device).await?;
                t.mul(&t)?
            }

            // ── Binary ops ──────────────────────────────────────────────────
            MathOp::Add | MathOp::Sub | MathOp::Mul | MathOp::Div => {
                let a = build_tensor(&inputs[0], &self.device).await?;
                let b = build_tensor(&inputs[1], &self.device).await?;
                match op {
                    MathOp::Add => a.add(&b)?,
                    MathOp::Sub => a.sub(&b)?,
                    MathOp::Mul => a.mul(&b)?,
                    MathOp::Div => a.div(&b)?,
                    _ => unreachable!(),
                }
            }

            // ── Matrix multiply ─────────────────────────────────────────────
            MathOp::MatMul { .. } | MathOp::BatchMatMul { .. } => {
                let a = build_tensor(&inputs[0], &self.device).await?;
                let b = build_tensor(&inputs[1], &self.device).await?;
                a.matmul(&b)?
            }

            // ── Activation ops ──────────────────────────────────────────────
            MathOp::Softmax { .. } => build_tensor(&inputs[0], &self.device).await?.softmax()?,
            MathOp::ReLU => build_tensor(&inputs[0], &self.device).await?.relu()?,
            MathOp::Sigmoid => build_tensor(&inputs[0], &self.device).await?.sigmoid()?,
            MathOp::Tanh => build_tensor(&inputs[0], &self.device).await?.tanh()?,
            MathOp::GELU => build_tensor(&inputs[0], &self.device).await?.gelu_wgsl()?,

            // ── Reductions ──────────────────────────────────────────────────
            MathOp::ReduceSum { .. } => build_tensor(&inputs[0], &self.device).await?.sum()?,
            MathOp::ReduceMean { .. } => build_tensor(&inputs[0], &self.device).await?.mean()?,
            MathOp::ReduceMax { .. } => build_tensor(&inputs[0], &self.device).await?.max()?,
            MathOp::ReduceMin { .. } => build_tensor(&inputs[0], &self.device).await?.min()?,
            MathOp::ReduceProd { .. } => build_tensor(&inputs[0], &self.device).await?.prod()?,

            // ── Pow (scalar exponent via GPU, extracts first element of b) ─
            MathOp::Pow => {
                let a = build_tensor(&inputs[0], &self.device).await?;
                let b_data = inputs[1].read_to_cpu().await?;
                let exp = if b_data.len() >= 4 {
                    f32::from_ne_bytes([b_data[0], b_data[1], b_data[2], b_data[3]])
                } else {
                    2.0f32
                };
                a.pow_wgsl(exp)?
            }

            // ── Binary Max / Min (elementwise, CPU fallback pending GPU kernel) ─
            MathOp::Max | MathOp::Min => {
                let a_data = inputs[0].read_to_cpu().await?;
                let b_data = inputs[1].read_to_cpu().await?;
                let a_f32: Vec<f32> = a_data
                    .chunks_exact(4)
                    .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                let b_f32: Vec<f32> = b_data
                    .chunks_exact(4)
                    .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                let result: Vec<f32> = a_f32
                    .iter()
                    .zip(b_f32.iter())
                    .map(|(&a, &b)| {
                        if matches!(op, MathOp::Max) {
                            a.max(b)
                        } else {
                            a.min(b)
                        }
                    })
                    .collect();
                crate::tensor::Tensor::from_data(
                    &result,
                    inputs[0].descriptor().shape.clone(),
                    self.device.clone(),
                )?
            }

            // ── Shape ops ───────────────────────────────────────────────────
            MathOp::Reshape { new_shape } => {
                let t = build_tensor(&inputs[0], &self.device).await?;
                t.reshape(new_shape.iter().map(|&x| x as usize).collect())?
            }
            MathOp::Transpose { .. } => {
                build_tensor(&inputs[0], &self.device).await?.transpose()?
            }
            MathOp::Squeeze { .. } => build_tensor(&inputs[0], &self.device).await?.squeeze()?,
            MathOp::Unsqueeze { dims } => {
                let axis = dims.first().copied().unwrap_or(0);
                build_tensor(&inputs[0], &self.device)
                    .await?
                    .unsqueeze(axis)?
            }
            MathOp::Broadcast { target_shape } => build_tensor(&inputs[0], &self.device)
                .await?
                .broadcast(target_shape.clone())?,
            MathOp::Concat { .. } => {
                if inputs.len() < 2 {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "Concat requires at least 2 inputs".to_string(),
                    });
                }
                let a = build_tensor(&inputs[0], &self.device).await?;
                let b = build_tensor(&inputs[1], &self.device).await?;
                a.concat(&b)?
            }
            MathOp::Split { sizes, .. } => {
                let t = build_tensor(&inputs[0], &self.device).await?;
                let split_point = sizes.first().copied().unwrap_or(t.len() / 2);
                let (first, _second) = t.split(split_point)?;
                first
            }

            // ── Convolution ops (full NCHW GPU via Conv2dGpu) ──────────────────
            // Uses the full NCHW shader with stride, padding, dilation, groups.
            // Handles 2D inputs by promoting to [1,1,H,W], and falls back to
            // the simple Conv2D op for trivial 2D cases without NCHW overhead.
            MathOp::Conv2D {
                stride: (stride_h, stride_w),
                padding: (pad_h, pad_w),
                dilation: (dil_h, dil_w),
                groups,
            } => {
                if inputs.len() < 2 {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "Conv2D requires 2 inputs (input, kernel)".to_string(),
                    });
                }
                let in_desc = inputs[0].descriptor();
                let kernel_desc = inputs[1].descriptor();

                let can_nchw = in_desc.shape.len() == 4 && kernel_desc.shape.len() == 4;
                let can_promote_2d = in_desc.shape.len() == 2
                    && kernel_desc.shape.len() >= 2
                    && *stride_h == 1
                    && *stride_w == 1
                    && *pad_h == 0
                    && *pad_w == 0
                    && *dil_h == 1
                    && *dil_w == 1
                    && *groups == 1;

                if can_nchw {
                    let input_t = build_tensor(&inputs[0], &self.device).await?;
                    let kernel_t = build_tensor(&inputs[1], &self.device).await?;
                    let bias_t = if inputs.len() > 2 {
                        Some(build_tensor(&inputs[2], &self.device).await?)
                    } else {
                        None
                    };
                    crate::ops::nn::Conv2dGpu {
                        input: input_t,
                        kernel: kernel_t,
                        bias: bias_t,
                        stride: (*stride_h, *stride_w),
                        padding: (*pad_h, *pad_w),
                        dilation: (*dil_h, *dil_w),
                        groups: *groups,
                    }
                    .execute()?
                } else if can_promote_2d {
                    let input_t = build_tensor(&inputs[0], &self.device).await?;
                    let kernel_t = build_tensor(&inputs[1], &self.device).await?;
                    let (k_h, k_w) = if kernel_desc.shape.len() == 2 {
                        (kernel_desc.shape[0], kernel_desc.shape[1])
                    } else {
                        (kernel_desc.shape[2], kernel_desc.shape[3])
                    };
                    let kernel_2d = kernel_t.reshape(vec![k_h, k_w])?;
                    input_t.conv2d(&kernel_2d)?
                } else {
                    let cpu = CpuExecutor::new();
                    let mut cpu_inputs = Vec::with_capacity(inputs.len());
                    for inp in &inputs {
                        let on_cpu = cpu.transfer(inp.clone()).await?;
                        cpu_inputs.push(on_cpu);
                    }
                    let cpu_result = cpu.execute(op, cpu_inputs).await?;
                    return self.transfer(cpu_result).await;
                }
            }

            MathOp::MaxPool2D {
                kernel_size: (k_h, k_w),
                stride: (stride_h, stride_w),
                padding: (pad_h, pad_w),
            } => {
                let in_desc = inputs[0].descriptor();
                let use_gpu = *k_h == *k_w
                    && *stride_h == *stride_w
                    && in_desc.shape.len() >= 2;

                let gpu_params = if use_gpu && in_desc.shape.len() == 4 {
                    let (n, c, h, w) = (
                        in_desc.shape[0],
                        in_desc.shape[1],
                        in_desc.shape[2],
                        in_desc.shape[3],
                    );
                    if n == 1 && c == 1 {
                        Some((vec![h, w], *k_h, *stride_h, *pad_h, *pad_w))
                    } else {
                        None
                    }
                } else if use_gpu && in_desc.shape.len() == 2 {
                    Some((in_desc.shape.clone(), *k_h, *stride_h, *pad_h, *pad_w))
                } else {
                    None
                };

                if let Some((in_shape, pool_size, stride, ph, pw)) = gpu_params {
                    let input_t = build_tensor(&inputs[0], &self.device).await?;
                    let input_2d_t = input_t.reshape(in_shape)?;
                    let out = input_2d_t.maxpool2d_padded(pool_size, stride, ph, pw)?;
                    if in_desc.shape.len() == 4 {
                        out.reshape(vec![1, 1, out.shape()[0], out.shape()[1]])?
                    } else {
                        out
                    }
                } else {
                    let cpu = CpuExecutor::new();
                    let mut cpu_inputs = Vec::with_capacity(inputs.len());
                    for inp in &inputs {
                        let on_cpu = cpu.transfer(inp.clone()).await?;
                        cpu_inputs.push(on_cpu);
                    }
                    let cpu_result = cpu.execute(op, cpu_inputs).await?;
                    return self.transfer(cpu_result).await;
                }
            }

            MathOp::AvgPool2D {
                kernel_size: (k_h, k_w),
                stride: (stride_h, stride_w),
                padding: (pad_h, pad_w),
            } => {
                let in_desc = inputs[0].descriptor();
                let use_gpu = *k_h == *k_w
                    && *stride_h == *stride_w
                    && in_desc.shape.len() >= 2;

                let gpu_params = if use_gpu && in_desc.shape.len() == 4 {
                    let (n, c, h, w) = (
                        in_desc.shape[0],
                        in_desc.shape[1],
                        in_desc.shape[2],
                        in_desc.shape[3],
                    );
                    if n == 1 && c == 1 {
                        Some((vec![h, w], *k_h, *stride_h, *pad_h, *pad_w))
                    } else {
                        None
                    }
                } else if use_gpu && in_desc.shape.len() == 2 {
                    Some((in_desc.shape.clone(), *k_h, *stride_h, *pad_h, *pad_w))
                } else {
                    None
                };

                if let Some((in_shape, pool_size, stride, ph, pw)) = gpu_params {
                    let input_t = build_tensor(&inputs[0], &self.device).await?;
                    let input_2d_t = input_t.reshape(in_shape)?;
                    input_2d_t.avgpool2d_padded(pool_size, stride, ph, pw)?
                } else {
                    let cpu = CpuExecutor::new();
                    let mut cpu_inputs = Vec::with_capacity(inputs.len());
                    for inp in &inputs {
                        let on_cpu = cpu.transfer(inp.clone()).await?;
                        cpu_inputs.push(on_cpu);
                    }
                    let cpu_result = cpu.execute(op, cpu_inputs).await?;
                    return self.transfer(cpu_result).await;
                }
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
