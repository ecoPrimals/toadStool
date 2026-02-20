//! TensorSession — Automatic Operation Batching
//!
//! **Problem**: Individual tensor operations have ~250 μs overhead each.
//! A chain of 100 operations = 25 ms of pure overhead.
//!
//! **Solution**: Sessions batch operations and execute together —
//! all ops are recorded into a single `CommandEncoder` and submitted once.
//!
//! ```rust,no_run
//! # use barracuda::prelude::*;
//! # async fn example() -> Result<()> {
//! let device = WgpuDevice::new().await?;
//!
//! // Create a session for batching
//! let mut session = TensorSession::new(&device);
//!
//! // Record operations (no GPU work yet)
//! let x  = session.tensor(&[1.0, 2.0, 3.0, 4.0])?;
//! let w  = session.tensor_with_shape(&[1.0, 0.0, 0.0, 1.0], vec![4, 1])?;
//! let h  = session.matmul(&x.reshape(vec![1, 4])?, &w)?;
//! let h2 = session.relu(&h)?;
//!
//! // Execute all operations in ONE command submission
//! session.run()?;
//!
//! let result = h2.to_vec()?;
//! # Ok(())
//! # }
//! ```
//!
//! **Supported ops**: `add`, `mul`, `fma`, `scale` (elementwise),
//! `matmul` (4-tier tiered), `relu`, `gelu`, `softmax`, `layer_norm`.
//!
//! **Performance**: N ops → 1×250 μs submit + N×~1 μs encode.
//! Absorbed from `neuralSpring` handoffs S-01 and S-11 (Feb 2026).

use crate::device::capabilities::DeviceCapabilities;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

// ─── Matmul tier selection (mirrors ops/matmul.rs thresholds) ────────────────

const MATMUL_SMALL_THRESHOLD: usize = 32;
const MATMUL_GPU_EVOLVED_THRESHOLD: usize = 256;

/// Tiered matmul shader selection — same logic as `ops::MatMul::select_tier`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatMulTier {
    /// Naive global-memory per-thread (tiny matrices)
    Naive,
    /// 16×16 single-buffer shared memory tiles (medium GPU)
    Tiled16,
    /// 32×32 double-buffered, fma, 8×4 micro-kernel (CPU / llvmpipe)
    CpuTiled32,
    /// 32×32 double-buffered, 2×2 micro-kernel (large GPU)
    GpuEvolved32,
}

impl MatMulTier {
    fn select(caps: &DeviceCapabilities, m: usize, n: usize) -> Self {
        if m < MATMUL_SMALL_THRESHOLD || n < MATMUL_SMALL_THRESHOLD {
            return Self::Naive;
        }
        if caps.device_type == wgpu::DeviceType::Cpu {
            return Self::CpuTiled32;
        }
        if m >= MATMUL_GPU_EVOLVED_THRESHOLD && n >= MATMUL_GPU_EVOLVED_THRESHOLD {
            Self::GpuEvolved32
        } else {
            Self::Tiled16
        }
    }

    fn dispatch(self, m: u32, n: u32) -> (u32, u32) {
        match self {
            Self::Naive                             => (m.div_ceil(16), n.div_ceil(16)),
            Self::Tiled16                           => (n.div_ceil(16), m.div_ceil(16)),
            Self::CpuTiled32 | Self::GpuEvolved32  => (n.div_ceil(32), m.div_ceil(32)),
        }
    }
}

// ─── MatMul uniform params struct (matches all four matmul WGSL shaders) ─────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct MatMulParams {
    m: u32,
    k: u32,
    n: u32,
    _padding: u32,
}

// ─── LayerNorm uniform params struct (matches layer_norm.wgsl) ───────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct LayerNormParams {
    size: u32,
    feature_size: u32,
    epsilon: f32,
}

/// Handle to a tensor within a session
///
/// This is a lightweight reference that tracks tensors created/computed
/// within a session. The actual data lives in GPU buffers.
#[derive(Debug, Clone)]
pub struct SessionTensor {
    /// Index into session's buffer registry
    buffer_id: usize,
    /// Shape of the tensor
    shape: Vec<usize>,
    /// Reference to the session's device
    device: Arc<WgpuDevice>,
    /// The actual buffer (available after session.run())
    buffer: Option<Arc<wgpu::Buffer>>,
}

impl SessionTensor {
    /// Get tensor shape
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Number of elements
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert to regular Tensor (after session.run())
    ///
    /// Note: This creates a deep copy of the data for safety.
    pub fn to_tensor(&self) -> Result<Tensor> {
        // Just read the data and create a new tensor
        let data = self.to_vec()?;
        Tensor::from_data(&data, self.shape.clone(), self.device.clone())
    }

    /// Read data back to CPU (after session.run())
    pub fn to_vec(&self) -> Result<Vec<f32>> {
        let buffer = self.buffer.as_ref().ok_or_else(|| {
            BarracudaError::execution_failed("Session not executed yet - call session.run() first")
        })?;

        self.device.read_buffer_f32(buffer, self.len())
    }
}

/// Recorded operation in a session.
#[derive(Debug)]
enum SessionOp {
    // ── Elementwise ──────────────────────────────────────────────────────────
    /// output = a + b
    Add { input_a: usize, input_b: usize, output: usize },
    /// output = a * b
    Mul { input_a: usize, input_b: usize, output: usize },
    /// output = a * b + c
    Fma { input_a: usize, input_b: usize, input_c: usize, output: usize },
    /// output = a * scalar
    Scale { input: usize, scalar: f32, output: usize },

    // ── Linear algebra ───────────────────────────────────────────────────────
    /// output[m×n] = a[m×k] × b[k×n] — 4-tier device-aware dispatch
    MatMul {
        input_a: usize,
        input_b: usize,
        output: usize,
        m: u32,
        k: u32,
        n: u32,
        tier: MatMulTier,
    },

    // ── Activations ──────────────────────────────────────────────────────────
    /// output = max(0, input)
    ReLU { input: usize, output: usize },
    /// output = input × Φ(input)  (tanh approximation)
    GELU { input: usize, output: usize },
    /// Row-wise softmax: output = exp(x) / Σexp(x)
    Softmax { input: usize, output: usize },

    // ── Normalisation ─────────────────────────────────────────────────────────
    /// Layer normalisation over last `feature_size` elements per row
    LayerNorm {
        input: usize,
        output: usize,
        feature_size: u32,
    },
}

/// Session for batching tensor operations
///
/// Operations are recorded without execution until `run()` is called.
/// This amortizes the ~250μs per-operation overhead across all operations.
pub struct TensorSession {
    device: Arc<WgpuDevice>,
    /// All buffers in the session (inputs and outputs)
    buffers: Vec<Arc<wgpu::Buffer>>,
    /// Shapes for each buffer
    shapes: Vec<Vec<usize>>,
    /// Recorded operations
    ops: Vec<SessionOp>,
    /// Optimal workgroup size (from calibration)
    workgroup_size: u32,
    /// Has the session been executed?
    executed: bool,
}

impl TensorSession {
    /// Create a new session for a device
    pub fn new(device: &WgpuDevice) -> Self {
        let wg_size = device.optimal_workgroup_size();
        Self {
            device: Arc::new(device.clone()),
            buffers: Vec::new(),
            shapes: Vec::new(),
            ops: Vec::new(),
            workgroup_size: wg_size,
            executed: false,
        }
    }

    /// Create session with explicit device Arc
    pub fn with_device(device: Arc<WgpuDevice>) -> Self {
        let wg_size = device.optimal_workgroup_size();
        Self {
            device,
            buffers: Vec::new(),
            shapes: Vec::new(),
            ops: Vec::new(),
            workgroup_size: wg_size,
            executed: false,
        }
    }

    /// Create a tensor from data within the session
    pub fn tensor(&mut self, data: &[f32]) -> Result<SessionTensor> {
        let shape = vec![data.len()];
        self.tensor_with_shape(data, shape)
    }

    /// Create a tensor with explicit shape
    pub fn tensor_with_shape(&mut self, data: &[f32], shape: Vec<usize>) -> Result<SessionTensor> {
        let expected_len: usize = shape.iter().product();
        if data.len() != expected_len {
            return Err(BarracudaError::invalid_shape(
                shape.clone(),
                vec![data.len()],
            ));
        }

        let buffer = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Session Input"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        let buffer_id = self.buffers.len();
        self.buffers.push(Arc::new(buffer));
        self.shapes.push(shape.clone());

        Ok(SessionTensor {
            buffer_id,
            shape,
            device: self.device.clone(),
            buffer: Some(self.buffers[buffer_id].clone()),
        })
    }

    /// Import an existing tensor into the session
    ///
    /// Note: This reads the tensor data and creates a new buffer in the session.
    pub fn import(&mut self, tensor: &Tensor) -> Result<SessionTensor> {
        let data = tensor.to_vec()?;
        self.tensor_with_shape(&data, tensor.shape().to_vec())
    }

    /// Allocate output buffer for an operation
    fn alloc_output(&mut self, shape: Vec<usize>) -> usize {
        let size = shape.iter().product::<usize>();
        let buffer = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Session Output"),
            size: (size * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let buffer_id = self.buffers.len();
        self.buffers.push(Arc::new(buffer));
        self.shapes.push(shape);
        buffer_id
    }

    /// Record add operation: output = a + b
    pub fn add(&mut self, a: &SessionTensor, b: &SessionTensor) -> Result<SessionTensor> {
        if a.shape() != b.shape() {
            return Err(BarracudaError::shape_mismatch(
                a.shape().to_vec(),
                b.shape().to_vec(),
            ));
        }

        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::Add {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            output: output_id,
        });

        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Record multiply operation: output = a * b
    pub fn mul(&mut self, a: &SessionTensor, b: &SessionTensor) -> Result<SessionTensor> {
        if a.shape() != b.shape() {
            return Err(BarracudaError::shape_mismatch(
                a.shape().to_vec(),
                b.shape().to_vec(),
            ));
        }

        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::Mul {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            output: output_id,
        });

        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Record fused multiply-add: output = a * b + c
    pub fn fma(
        &mut self,
        a: &SessionTensor,
        b: &SessionTensor,
        c: &SessionTensor,
    ) -> Result<SessionTensor> {
        if a.shape() != b.shape() {
            return Err(BarracudaError::shape_mismatch(
                a.shape().to_vec(),
                b.shape().to_vec(),
            ));
        }
        if a.shape() != c.shape() {
            return Err(BarracudaError::shape_mismatch(
                a.shape().to_vec(),
                c.shape().to_vec(),
            ));
        }

        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::Fma {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            input_c: c.buffer_id,
            output: output_id,
        });

        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Record scale operation: output = a * scalar
    pub fn scale(&mut self, a: &SessionTensor, scalar: f32) -> Result<SessionTensor> {
        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::Scale {
            input: a.buffer_id,
            scalar,
            output: output_id,
        });

        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    // ── ML operations ─────────────────────────────────────────────────────────

    /// Record matrix multiply: output\[m×n\] = a\[m×k\] × b\[k×n\].
    ///
    /// Tier selection (device-aware, 4-tier):
    /// - tiny matrices → naive
    /// - CPU / llvmpipe → 32×32 double-buffered cpu-tiled
    /// - small GPU → 16×16 tiled
    /// - large GPU (≥256×256) → 32×32 double-buffered gpu-evolved
    ///
    /// Absorbed from `neuralSpring` S-02 handoff (46–104× faster than per-op).
    pub fn matmul(&mut self, a: &SessionTensor, b: &SessionTensor) -> Result<SessionTensor> {
        if a.shape.len() != 2 || b.shape.len() != 2 {
            return Err(BarracudaError::invalid_shape(
                a.shape.clone(),
                b.shape.clone(),
            ));
        }
        let m = a.shape[0];
        let k = a.shape[1];
        if b.shape[0] != k {
            return Err(BarracudaError::shape_mismatch(
                vec![m, k],
                b.shape.clone(),
            ));
        }
        let n = b.shape[1];

        let caps = DeviceCapabilities::from_device(&self.device);
        let tier = MatMulTier::select(&caps, m, n);

        let output_id = self.alloc_output(vec![m, n]);
        self.ops.push(SessionOp::MatMul {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            output: output_id,
            m: m as u32,
            k: k as u32,
            n: n as u32,
            tier,
        });

        Ok(SessionTensor {
            buffer_id: output_id,
            shape: vec![m, n],
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Record ReLU activation: output = max(0, input).
    pub fn relu(&mut self, a: &SessionTensor) -> Result<SessionTensor> {
        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::ReLU {
            input: a.buffer_id,
            output: output_id,
        });
        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Record GELU activation: output = x × Φ(x).
    pub fn gelu(&mut self, a: &SessionTensor) -> Result<SessionTensor> {
        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::GELU {
            input: a.buffer_id,
            output: output_id,
        });
        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Record row-wise softmax: output = exp(x) / Σexp(x).
    pub fn softmax(&mut self, a: &SessionTensor) -> Result<SessionTensor> {
        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::Softmax {
            input: a.buffer_id,
            output: output_id,
        });
        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Record layer normalisation over the last `feature_size` elements per row.
    ///
    /// Normalises each row of `a` to zero mean and unit variance, then applies
    /// learnable `γ=1` and `β=0` (no affine transform in this pass — wire a
    /// subsequent scale+add if affine parameters are needed).
    pub fn layer_norm(&mut self, a: &SessionTensor, feature_size: usize) -> Result<SessionTensor> {
        let total: usize = a.shape.iter().product();
        if total % feature_size != 0 {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "layer_norm: total elements {total} not divisible by feature_size {feature_size}"
                ),
            });
        }
        let output_id = self.alloc_output(a.shape.clone());
        self.ops.push(SessionOp::LayerNorm {
            input: a.buffer_id,
            output: output_id,
            feature_size: feature_size as u32,
        });
        Ok(SessionTensor {
            buffer_id: output_id,
            shape: a.shape.clone(),
            device: self.device.clone(),
            buffer: Some(self.buffers[output_id].clone()),
        })
    }

    /// Reshape a `SessionTensor` — metadata-only, no GPU work.
    ///
    /// The buffer is shared (Arc clone); only the shape changes.
    pub fn reshape(
        &mut self,
        a: &SessionTensor,
        new_shape: Vec<usize>,
    ) -> Result<SessionTensor> {
        let old_len: usize = a.shape.iter().product();
        let new_len: usize = new_shape.iter().product();
        if old_len != new_len {
            return Err(BarracudaError::shape_mismatch(
                new_shape,
                a.shape.clone(),
            ));
        }
        // Re-use the same buffer — just change the recorded shape.
        Ok(SessionTensor {
            buffer_id: a.buffer_id,
            shape: new_shape,
            device: self.device.clone(),
            buffer: a.buffer.clone(),
        })
    }

    /// Number of recorded operations
    pub fn num_ops(&self) -> usize {
        self.ops.len()
    }

    /// Execute all recorded operations
    ///
    /// This is where the performance magic happens - all operations
    /// are batched into a single command buffer submission.
    pub fn run(&mut self) -> Result<()> {
        if self.ops.is_empty() {
            return Ok(());
        }

        // Pre-compile shaders — compiled once per run(), shared across all ops
        // of the same type.  This amortises SPIR-V translation for repeated ops.
        let add_shader   = self.compile_inline_shader("add");
        let mul_shader   = self.compile_inline_shader("mul");
        let fma_shader   = self.compile_inline_shader("fma");
        let scale_shader = self.compile_inline_shader("scale");
        // ML op shaders compiled from `include_str!` sources:
        let relu_pl  = self.compile_auto_pipeline(
            include_str!("shaders/activation/relu.wgsl"), "Session ReLU");
        let gelu_pl  = self.compile_auto_pipeline(
            include_str!("shaders/activation/gelu.wgsl"), "Session GELU");
        let sfmx_pl  = self.compile_auto_pipeline(
            include_str!("shaders/activation/softmax_simple.wgsl"), "Session Softmax");
        let lnrm_pl  = self.compile_auto_pipeline(
            include_str!("shaders/norm/layer_norm.wgsl"), "Session LayerNorm");
        // Matmul: compile all tiers upfront; only the needed ones are used.
        let mm_naive_pl  = self.compile_auto_pipeline(
            include_str!("shaders/math/matmul.wgsl"), "Session MatMul Naive");
        let mm_t16_pl    = self.compile_auto_pipeline(
            include_str!("shaders/math/matmul_tiled.wgsl"), "Session MatMul Tiled16");
        let mm_cpu_pl    = self.compile_auto_pipeline(
            include_str!("shaders/math/matmul_cpu_tiled.wgsl"), "Session MatMul CpuTiled32");
        let mm_gpu_pl    = self.compile_auto_pipeline(
            include_str!("shaders/math/matmul_gpu_evolved.wgsl"), "Session MatMul GpuEvolved32");

        // Create single command encoder for ALL operations
        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("TensorSession Batch"),
                });

        // Encode all operations into the single encoder
        for op in &self.ops {
            match op {
                // ── Elementwise ──────────────────────────────────────────────
                SessionOp::Add { input_a, input_b, output } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_binary_op(
                        &mut encoder, &add_shader,
                        &self.buffers[*input_a], &self.buffers[*input_b],
                        &self.buffers[*output], size,
                    );
                }
                SessionOp::Mul { input_a, input_b, output } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_binary_op(
                        &mut encoder, &mul_shader,
                        &self.buffers[*input_a], &self.buffers[*input_b],
                        &self.buffers[*output], size,
                    );
                }
                SessionOp::Fma { input_a, input_b, input_c, output } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_ternary_op(
                        &mut encoder, &fma_shader,
                        &self.buffers[*input_a], &self.buffers[*input_b],
                        &self.buffers[*input_c], &self.buffers[*output], size,
                    );
                }
                SessionOp::Scale { input, scalar, output } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_scale_op(
                        &mut encoder, &scale_shader,
                        &self.buffers[*input], *scalar,
                        &self.buffers[*output], size,
                    );
                }

                // ── Matrix multiply ───────────────────────────────────────────
                SessionOp::MatMul { input_a, input_b, output, m, k, n, tier } => {
                    let pipeline = match tier {
                        MatMulTier::Naive       => &mm_naive_pl,
                        MatMulTier::Tiled16     => &mm_t16_pl,
                        MatMulTier::CpuTiled32  => &mm_cpu_pl,
                        MatMulTier::GpuEvolved32=> &mm_gpu_pl,
                    };
                    let params_buf = self.device.device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Session MatMul Params"),
                            contents: bytemuck::bytes_of(&MatMulParams {
                                m: *m, k: *k, n: *n, _padding: 0,
                            }),
                            usage: wgpu::BufferUsages::UNIFORM,
                        },
                    );
                    let bg = self.auto_bind_group(
                        pipeline,
                        &[
                            &self.buffers[*input_a],
                            &self.buffers[*input_b],
                            &self.buffers[*output],
                            &params_buf,
                        ],
                    );
                    let (wg_x, wg_y) = tier.dispatch(*m, *n);
                    let mut pass = encoder.begin_compute_pass(
                        &wgpu::ComputePassDescriptor::default());
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.dispatch_workgroups(wg_x, wg_y, 1);
                }

                // ── Activations ───────────────────────────────────────────────
                SessionOp::ReLU { input, output } => {
                    // relu.wgsl: (input, output) — no uniform, uses arrayLength
                    let bg = self.auto_bind_group(
                        &relu_pl,
                        &[&self.buffers[*input], &self.buffers[*output]],
                    );
                    let size = self.shapes[*output].iter().product::<usize>() as u32;
                    let mut pass = encoder.begin_compute_pass(
                        &wgpu::ComputePassDescriptor::default());
                    pass.set_pipeline(&relu_pl);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.dispatch_workgroups(size.div_ceil(256), 1, 1);
                }
                SessionOp::GELU { input, output } => {
                    // gelu.wgsl: (input, output, size: u32 uniform)
                    let size = self.shapes[*output].iter().product::<usize>() as u32;
                    let size_buf = self.make_uniform_u32(size, "Session GELU Size");
                    let bg = self.auto_bind_group(
                        &gelu_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &size_buf],
                    );
                    let mut pass = encoder.begin_compute_pass(
                        &wgpu::ComputePassDescriptor::default());
                    pass.set_pipeline(&gelu_pl);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.dispatch_workgroups(size.div_ceil(256), 1, 1);
                }
                SessionOp::Softmax { input, output } => {
                    // softmax_simple.wgsl: (input, output, Params { size })
                    let size = self.shapes[*output].iter().product::<usize>() as u32;
                    let params_buf = self.make_uniform_u32(size, "Session Softmax Params");
                    let bg = self.auto_bind_group(
                        &sfmx_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &params_buf],
                    );
                    let mut pass = encoder.begin_compute_pass(
                        &wgpu::ComputePassDescriptor::default());
                    pass.set_pipeline(&sfmx_pl);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.dispatch_workgroups(1, 1, 1);  // single-workgroup reduction
                }

                // ── Normalisation ─────────────────────────────────────────────
                SessionOp::LayerNorm { input, output, feature_size } => {
                    // layer_norm.wgsl: (input, output, Params { size, feature_size, epsilon })
                    let total = self.shapes[*output].iter().product::<usize>() as u32;
                    let params_buf = self.device.device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Session LayerNorm Params"),
                            contents: bytemuck::bytes_of(&LayerNormParams {
                                size: total,
                                feature_size: *feature_size,
                                epsilon: 1e-5,
                            }),
                            usage: wgpu::BufferUsages::UNIFORM,
                        },
                    );
                    let bg = self.auto_bind_group(
                        &lnrm_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &params_buf],
                    );
                    let rows = (total / feature_size).max(1);
                    let mut pass = encoder.begin_compute_pass(
                        &wgpu::ComputePassDescriptor::default());
                    pass.set_pipeline(&lnrm_pl);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.dispatch_workgroups(rows.div_ceil(256), 1, 1);
                }
            }
        }

        // Single submission for ALL operations
        self.device.queue.submit(Some(encoder.finish()));
        self.device.device.poll(wgpu::Maintain::Wait);

        self.executed = true;
        Ok(())
    }

    /// Reset session for reuse (clears operations but keeps device)
    pub fn reset(&mut self) {
        self.buffers.clear();
        self.shapes.clear();
        self.ops.clear();
        self.executed = false;
    }

    /// Compile a compute pipeline with auto-derived bind group layout.
    ///
    /// Using `layout: None` lets wgpu derive the BGL from the shader — no manual
    /// `BindGroupLayoutDescriptor` needed.  The bind group is then created via
    /// `pipeline.get_bind_group_layout(0)` in `auto_bind_group`.
    fn compile_auto_pipeline(&self, src: &str, label: &str) -> wgpu::ComputePipeline {
        let module = self.device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });
        self.device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: None,
            module: &module,
            entry_point: "main",
            cache: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        })
    }

    /// Create a bind group matching a pipeline's auto-derived layout at group 0.
    ///
    /// Buffers are bound in order: slot 0 = `buffers[0]`, 1 = `buffers[1]`, …
    fn auto_bind_group(
        &self,
        pipeline: &wgpu::ComputePipeline,
        buffers: &[&wgpu::Buffer],
    ) -> wgpu::BindGroup {
        let layout = pipeline.get_bind_group_layout(0);
        let entries: Vec<wgpu::BindGroupEntry<'_>> = buffers
            .iter()
            .enumerate()
            .map(|(i, buf)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buf.as_entire_binding(),
            })
            .collect();
        self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        })
    }

    /// Create a uniform buffer containing a single `u32`.
    fn make_uniform_u32(&self, value: u32, label: &str) -> wgpu::Buffer {
        self.device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::bytes_of(&value),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }

    fn compile_inline_shader(&self, op_type: &str) -> wgpu::ShaderModule {
        let source = match op_type {
            "add" => format!(
                r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{ return; }}
    output[idx] = a[idx] + b[idx];
}}
"#,
                self.workgroup_size
            ),
            "mul" => format!(
                r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{ return; }}
    output[idx] = a[idx] * b[idx];
}}
"#,
                self.workgroup_size
            ),
            "fma" => format!(
                r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read> c: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{ return; }}
    output[idx] = fma(a[idx], b[idx], c[idx]);
}}
"#,
                self.workgroup_size
            ),
            "scale" => format!(
                r#"
struct Params {{ scalar: f32 }}
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{ return; }}
    output[idx] = a[idx] * params.scalar;
}}
"#,
                self.workgroup_size
            ),
            _ => unreachable!("Unknown op type: {op_type} - internal invariant violation"),
        };

        self.device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(op_type),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    fn encode_binary_op(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shader: &wgpu::ShaderModule,
        input_a: &wgpu::Buffer,
        input_b: &wgpu::Buffer,
        output: &wgpu::Buffer,
        size: usize,
    ) {
        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_a.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: input_b.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output.as_entire_binding(),
                    },
                ],
            });

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    module: shader,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let workgroups = (size as u32).div_ceil(self.workgroup_size).min(65535);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }

    fn encode_ternary_op(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shader: &wgpu::ShaderModule,
        input_a: &wgpu::Buffer,
        input_b: &wgpu::Buffer,
        input_c: &wgpu::Buffer,
        output: &wgpu::Buffer,
        size: usize,
    ) {
        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_a.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: input_b.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: input_c.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output.as_entire_binding(),
                    },
                ],
            });

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    module: shader,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let workgroups = (size as u32).div_ceil(self.workgroup_size).min(65535);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }

    fn encode_scale_op(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shader: &wgpu::ShaderModule,
        input: &wgpu::Buffer,
        scalar: f32,
        output: &wgpu::Buffer,
        size: usize,
    ) {
        // Create uniform buffer for scalar
        let scalar_buffer = self.device.create_uniform_buffer("scalar", &scalar);

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: scalar_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output.as_entire_binding(),
                    },
                ],
            });

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    module: shader,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let workgroups = (size as u32).div_ceil(self.workgroup_size).min(65535);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool;

    #[tokio::test]
    async fn test_session_basic() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };

        let mut session = TensorSession::new(&device);

        let a = session.tensor(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        let b = session.tensor(&[5.0, 6.0, 7.0, 8.0]).unwrap();
        let c = session.add(&a, &b).unwrap();

        assert_eq!(session.num_ops(), 1);

        session.run().unwrap();

        let result = c.to_vec().unwrap();
        assert_eq!(result, vec![6.0, 8.0, 10.0, 12.0]);
    }

    #[tokio::test]
    async fn test_session_chain() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };

        let mut session = TensorSession::new(&device);

        let a = session.tensor(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        let b = session.tensor(&[2.0, 2.0, 2.0, 2.0]).unwrap();

        // Chain: (a + b) * b = [3, 4, 5, 6] * [2, 2, 2, 2] = [6, 8, 10, 12]
        let c = session.add(&a, &b).unwrap();
        let d = session.mul(&c, &b).unwrap();

        assert_eq!(session.num_ops(), 2);

        session.run().unwrap();

        let result = d.to_vec().unwrap();
        assert_eq!(result, vec![6.0, 8.0, 10.0, 12.0]);
    }

    #[tokio::test]
    async fn test_session_fma() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };

        let mut session = TensorSession::new(&device);

        let a = session.tensor(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        let b = session.tensor(&[2.0, 2.0, 2.0, 2.0]).unwrap();
        let c = session.tensor(&[10.0, 10.0, 10.0, 10.0]).unwrap();

        // FMA: a * b + c = [1, 2, 3, 4] * [2, 2, 2, 2] + [10, 10, 10, 10]
        //                = [2, 4, 6, 8] + [10, 10, 10, 10] = [12, 14, 16, 18]
        let d = session.fma(&a, &b, &c).unwrap();

        session.run().unwrap();

        let result = d.to_vec().unwrap();
        assert_eq!(result, vec![12.0, 14.0, 16.0, 18.0]);
    }

    // ── ML op tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_session_matmul_2x2() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut session = TensorSession::new(&device);

        // A = [[1, 2], [3, 4]]  B = [[5, 6], [7, 8]]
        // C = A×B = [[19, 22], [43, 50]]
        let a = session.tensor_with_shape(&[1.0, 2.0, 3.0, 4.0], vec![2, 2]).unwrap();
        let b = session.tensor_with_shape(&[5.0, 6.0, 7.0, 8.0], vec![2, 2]).unwrap();
        let c = session.matmul(&a, &b).unwrap();

        assert_eq!(c.shape(), &[2, 2]);
        session.run().unwrap();

        let result = c.to_vec().unwrap();
        let expected = [19.0_f32, 22.0, 43.0, 50.0];
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 0.01, "matmul mismatch: {r} vs {e}");
        }
    }

    #[tokio::test]
    async fn test_session_relu() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut session = TensorSession::new(&device);

        let a = session.tensor(&[-2.0, -1.0, 0.0, 1.0, 2.0]).unwrap();
        let b = session.relu(&a).unwrap();

        session.run().unwrap();

        let result = b.to_vec().unwrap();
        assert_eq!(result, vec![0.0, 0.0, 0.0, 1.0, 2.0]);
    }

    #[tokio::test]
    async fn test_session_gelu() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut session = TensorSession::new(&device);

        // GELU(0) = 0.0, GELU(large) ≈ large
        let a = session.tensor(&[0.0_f32, 10.0]).unwrap();
        let b = session.gelu(&a).unwrap();

        session.run().unwrap();

        let result = b.to_vec().unwrap();
        assert!((result[0]).abs() < 0.01, "GELU(0) should be ~0, got {}", result[0]);
        assert!((result[1] - 10.0).abs() < 0.1, "GELU(10) should be ~10, got {}", result[1]);
    }

    #[tokio::test]
    async fn test_session_softmax() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut session = TensorSession::new(&device);

        let a = session.tensor(&[1.0_f32, 2.0, 3.0]).unwrap();
        let b = session.softmax(&a).unwrap();

        session.run().unwrap();

        let result = b.to_vec().unwrap();
        // Softmax outputs must sum to 1 and be increasing
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "softmax sum should be 1.0, got {sum}");
        assert!(result[0] < result[1] && result[1] < result[2], "softmax should be monotone");
    }

    #[tokio::test]
    async fn test_session_layer_norm() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut session = TensorSession::new(&device);

        // Two rows of [1, 2, 3, 4] — each row should normalise to ~0 mean
        let data = [1.0_f32, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0];
        let a = session.tensor_with_shape(&data, vec![2, 4]).unwrap();
        let b = session.layer_norm(&a, 4).unwrap();

        session.run().unwrap();

        let result = b.to_vec().unwrap();
        assert_eq!(result.len(), 8);
        // Each row should have mean ≈ 0
        for row in 0..2 {
            let mean: f32 = result[row*4..row*4+4].iter().sum::<f32>() / 4.0;
            assert!(mean.abs() < 0.05, "layer_norm row {row} mean should be ~0, got {mean}");
        }
    }

    #[tokio::test]
    async fn test_session_mlp_fused() {
        // End-to-end MLP: linear → relu → linear, all in one session
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut session = TensorSession::new(&device);

        // input [1×2], W1 [2×4] (expand), W2 [4×1] (project back)
        let x  = session.tensor_with_shape(&[1.0_f32, 2.0], vec![1, 2]).unwrap();
        let w1 = session
            .tensor_with_shape(&[1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0], vec![2, 4])
            .unwrap();
        let w2 = session
            .tensor_with_shape(&[1.0, 1.0, 1.0, 1.0], vec![4, 1])
            .unwrap();

        let h1 = session.matmul(&x, &w1).unwrap();   // [1×4]
        let h2 = session.relu(&h1).unwrap();          // [1×4]
        let y  = session.matmul(&h2, &w2).unwrap();   // [1×1]

        assert_eq!(session.num_ops(), 3);
        session.run().unwrap();

        let result = y.to_vec().unwrap();
        // W1×x = [1,2,1,2]; relu = [1,2,1,2]; W2×relu = 1+2+1+2 = 6
        assert!((result[0] - 6.0).abs() < 0.1, "MLP output should be 6.0, got {}", result[0]);
    }
}
