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
//! let w  = session.tensor_with_shape(&[1.0, 0.0, 0.0, 1.0], &[4, 1])?;
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
//! `matmul` (4-tier tiered), `relu`, `gelu`, `softmax`, `layer_norm`,
//! `attention`, `head_split`, `head_concat`, `reshape`.
//!
//! **Performance**: N ops → 1×250 μs submit + N×~1 μs encode.
//! All pipelines compiled **once** at construction — `run()` is encode-only.
//!
//! Absorbed from `neuralSpring` handoffs S-01 and S-11 (Feb 2026).

mod pipelines;
mod tensor;
mod types;

pub use tensor::SessionTensor;

use crate::device::capabilities::DeviceCapabilities;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::bytes_of;
use pipelines::SessionPipelines;
use std::sync::Arc;
use types::{
    AttentionParams, HeadReshapeParams, LayerNormParams, MatMulParams, MatMulTier, ScaleParams,
    SessionOp,
};
use wgpu::util::DeviceExt;

// ─── TensorSession ─────────────────────────────────────────────────────────────

/// Session for batching tensor operations.
///
/// Operations are recorded without execution until `run()` is called.
/// This amortises the ~250 μs per-operation overhead across all operations.
///
/// Pipelines are compiled **once** at session construction.  Subsequent
/// `run()` calls pay only bind-group creation and dispatch encoding —
/// no SPIR-V translation.  Use `reset()` to clear recorded ops without
/// discarding pipelines.
pub struct TensorSession {
    device: Arc<WgpuDevice>,
    buffers: Vec<Arc<wgpu::Buffer>>,
    shapes: Vec<Vec<usize>>,
    ops: Vec<SessionOp>,
    workgroup_size: u32,
    executed: bool,
    pipelines: SessionPipelines,
}

impl TensorSession {
    /// Create a new session — compiles all pipelines once.
    pub fn new(device: &WgpuDevice) -> Self {
        let wg = device.optimal_workgroup_size();
        Self {
            device: Arc::new(device.clone()),
            buffers: Vec::new(),
            shapes: Vec::new(),
            ops: Vec::new(),
            workgroup_size: wg,
            executed: false,
            pipelines: SessionPipelines::build(&device.device, wg),
        }
    }

    /// Create a session with an explicit device `Arc`.
    pub fn with_device(device: Arc<WgpuDevice>) -> Self {
        let wg = device.optimal_workgroup_size();
        let pls = SessionPipelines::build(&device.device, wg);
        Self {
            device,
            buffers: Vec::new(),
            shapes: Vec::new(),
            ops: Vec::new(),
            workgroup_size: wg,
            executed: false,
            pipelines: pls,
        }
    }

    /// Clear recorded ops.  Compiled pipelines and allocated buffers are retained.
    pub fn reset(&mut self) {
        self.ops.clear();
        self.executed = false;
    }

    /// Number of recorded operations.
    pub fn num_ops(&self) -> usize {
        self.ops.len()
    }

    // ── Input ingestion ───────────────────────────────────────────────────────

    /// Upload a flat slice to a 1-D session tensor.
    pub fn tensor(&mut self, data: &[f32]) -> Result<SessionTensor> {
        self.tensor_with_shape(data, &[data.len()])
    }

    /// Upload data with an explicit shape (accepts slice to avoid cloning).
    pub fn tensor_with_shape(&mut self, data: &[f32], shape: &[usize]) -> Result<SessionTensor> {
        let expected: usize = shape.iter().product();
        if data.len() != expected {
            return Err(BarracudaError::invalid_shape(
                shape.to_vec(),
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
        let id = self.register_buffer(Arc::new(buffer), shape);
        Ok(self.make_tensor(id, shape))
    }

    /// Import an existing `Tensor` (reads to CPU, re-uploads — preserves interface).
    pub fn import(&mut self, tensor: &Tensor) -> Result<SessionTensor> {
        let data = tensor.to_vec()?;
        self.tensor_with_shape(&data, tensor.shape())
    }

    // ── Elementwise ops ───────────────────────────────────────────────────────

    /// `output = a + b`
    pub fn add(&mut self, a: &SessionTensor, b: &SessionTensor) -> Result<SessionTensor> {
        self.check_same_shape(a, b)?;
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::Add {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            output: out,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    /// `output = a * b`
    pub fn mul(&mut self, a: &SessionTensor, b: &SessionTensor) -> Result<SessionTensor> {
        self.check_same_shape(a, b)?;
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::Mul {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            output: out,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    /// `output = a * b + c`
    pub fn fma(
        &mut self,
        a: &SessionTensor,
        b: &SessionTensor,
        c: &SessionTensor,
    ) -> Result<SessionTensor> {
        self.check_same_shape(a, b)?;
        self.check_same_shape(a, c)?;
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::Fma {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            input_c: c.buffer_id,
            output: out,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    /// `output = a * scalar`
    pub fn scale(&mut self, a: &SessionTensor, scalar: f32) -> Result<SessionTensor> {
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::Scale {
            input: a.buffer_id,
            scalar,
            output: out,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    // ── Linear algebra ────────────────────────────────────────────────────────

    /// `output[m×n] = a[m×k] × b[k×n]` — 4-tier device-aware matmul.
    pub fn matmul(&mut self, a: &SessionTensor, b: &SessionTensor) -> Result<SessionTensor> {
        if a.shape.len() != 2 || b.shape.len() != 2 {
            return Err(BarracudaError::invalid_shape(
                a.shape.clone(),
                b.shape.clone(),
            ));
        }
        let (m, k) = (a.shape[0], a.shape[1]);
        if b.shape[0] != k {
            return Err(BarracudaError::shape_mismatch(vec![m, k], b.shape.clone()));
        }
        let n = b.shape[1];
        let caps = DeviceCapabilities::from_device(&self.device);
        let tier = MatMulTier::select(&caps, m, n);
        let out_shape = [m, n];
        let out = self.alloc_output(&out_shape);
        self.ops.push(SessionOp::MatMul {
            input_a: a.buffer_id,
            input_b: b.buffer_id,
            output: out,
            m: m as u32,
            k: k as u32,
            n: n as u32,
            tier,
        });
        Ok(self.make_tensor(out, &out_shape))
    }

    // ── Activations ───────────────────────────────────────────────────────────

    /// `output = max(0, input)`
    pub fn relu(&mut self, a: &SessionTensor) -> Result<SessionTensor> {
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::ReLU {
            input: a.buffer_id,
            output: out,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    /// `output = x × Φ(x)` (tanh-approximation GELU)
    pub fn gelu(&mut self, a: &SessionTensor) -> Result<SessionTensor> {
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::Gelu {
            input: a.buffer_id,
            output: out,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    /// Row-wise softmax: `output = exp(x) / Σexp(x)`
    pub fn softmax(&mut self, a: &SessionTensor) -> Result<SessionTensor> {
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::Softmax {
            input: a.buffer_id,
            output: out,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    // ── Normalisation ─────────────────────────────────────────────────────────

    /// Layer normalisation over the last `feature_size` elements per row.
    pub fn layer_norm(&mut self, a: &SessionTensor, feature_size: usize) -> Result<SessionTensor> {
        let total: usize = a.shape.iter().product();
        if !total.is_multiple_of(feature_size) {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "layer_norm: total {total} not divisible by feature_size {feature_size}"
                ),
            });
        }
        let out = self.alloc_output(&a.shape);
        self.ops.push(SessionOp::LayerNorm {
            input: a.buffer_id,
            output: out,
            feature_size: feature_size as u32,
        });
        Ok(self.make_tensor(out, &a.shape))
    }

    // ── Shape ops ─────────────────────────────────────────────────────────────

    /// Reshape — metadata-only, no GPU work.
    pub fn reshape(&mut self, a: &SessionTensor, new_shape: Vec<usize>) -> Result<SessionTensor> {
        let old_len: usize = a.shape.iter().product();
        let new_len: usize = new_shape.iter().product();
        if old_len != new_len {
            return Err(BarracudaError::shape_mismatch(new_shape, a.shape.clone()));
        }
        Ok(SessionTensor {
            buffer_id: a.buffer_id,
            shape: new_shape,
            device: self.device.clone(),
            buffer: a.buffer.clone(),
        })
    }

    // ── Attention ops ──────────────────────────────────────────────────────────

    /// Reshape `[B, S, H*D]` → `[B, H, S, D]` for multi-head attention.
    pub fn head_split(
        &mut self,
        a: &SessionTensor,
        batch_size: usize,
        seq_len: usize,
        n_heads: usize,
        head_dim: usize,
    ) -> Result<SessionTensor> {
        let expected = batch_size * seq_len * n_heads * head_dim;
        if a.len() != expected {
            return Err(BarracudaError::InvalidInput {
                message: format!("head_split: input len {} ≠ B×S×H×D={expected}", a.len()),
            });
        }
        let out_shape = [batch_size, n_heads, seq_len, head_dim];
        let out = self.alloc_output(&out_shape);
        self.ops.push(SessionOp::HeadSplit {
            input: a.buffer_id,
            output: out,
            batch_size: batch_size as u32,
            seq_len: seq_len as u32,
            num_heads: n_heads as u32,
            head_dim: head_dim as u32,
        });
        Ok(self.make_tensor(out, &out_shape))
    }

    /// Reshape `[B, H, S, D]` → `[B, S, H*D]` after multi-head attention.
    pub fn head_concat(
        &mut self,
        a: &SessionTensor,
        batch_size: usize,
        seq_len: usize,
        n_heads: usize,
        head_dim: usize,
    ) -> Result<SessionTensor> {
        let expected = batch_size * n_heads * seq_len * head_dim;
        if a.len() != expected {
            return Err(BarracudaError::InvalidInput {
                message: format!("head_concat: input len {} ≠ B×H×S×D={expected}", a.len()),
            });
        }
        let out_shape = [batch_size, seq_len, n_heads * head_dim];
        let out = self.alloc_output(&out_shape);
        self.ops.push(SessionOp::HeadConcat {
            input: a.buffer_id,
            output: out,
            batch_size: batch_size as u32,
            seq_len: seq_len as u32,
            num_heads: n_heads as u32,
            head_dim: head_dim as u32,
        });
        Ok(self.make_tensor(out, &out_shape))
    }

    /// Scaled dot-product attention: `output = softmax(QK^T / √d) · V`
    ///
    /// Q/K/V must be `[B, H, S, D]` (use `head_split` after projection).
    /// Encoded as 3 passes in a single encoder batch.
    pub fn attention(
        &mut self,
        q: &SessionTensor,
        k: &SessionTensor,
        v: &SessionTensor,
        batch_size: usize,
        n_heads: usize,
        seq_len: usize,
        head_dim: usize,
    ) -> Result<SessionTensor> {
        let expected = batch_size * n_heads * seq_len * head_dim;
        for (name, t) in [("q", q), ("k", k), ("v", v)] {
            if t.len() != expected {
                return Err(BarracudaError::InvalidInput {
                    message: format!("attention: {name} len {} ≠ B×H×S×D={expected}", t.len()),
                });
            }
        }
        let out_shape = [batch_size, n_heads, seq_len, head_dim];
        let out = self.alloc_output(&out_shape);
        self.ops.push(SessionOp::Attention {
            q: q.buffer_id,
            k: k.buffer_id,
            v: v.buffer_id,
            output: out,
            batch_size: batch_size as u32,
            num_heads: n_heads as u32,
            seq_len: seq_len as u32,
            head_dim: head_dim as u32,
        });
        Ok(self.make_tensor(out, &out_shape))
    }

    // ── Execution ─────────────────────────────────────────────────────────────

    /// Execute all recorded operations in a single GPU command submission.
    ///
    /// Pipelines are pre-compiled at construction — this call pays only
    /// bind-group creation and dispatch encoding (no SPIR-V translation).
    pub fn run(&mut self) -> Result<()> {
        if self.ops.is_empty() {
            return Ok(());
        }

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("TensorSession Batch"),
                });

        for op in &self.ops {
            match op {
                // ── Elementwise (pre-compiled pipelines, zero alloc) ──────────
                SessionOp::Add {
                    input_a,
                    input_b,
                    output,
                } => {
                    let size = self.elem_count(*output) as u32;
                    let bg = self.auto_bind_group(
                        &self.pipelines.add_pl,
                        &[
                            &self.buffers[*input_a],
                            &self.buffers[*input_b],
                            &self.buffers[*output],
                        ],
                    );
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.add_pl,
                        &bg,
                        size.div_ceil(self.workgroup_size),
                    );
                }
                SessionOp::Mul {
                    input_a,
                    input_b,
                    output,
                } => {
                    let size = self.elem_count(*output) as u32;
                    let bg = self.auto_bind_group(
                        &self.pipelines.mul_pl,
                        &[
                            &self.buffers[*input_a],
                            &self.buffers[*input_b],
                            &self.buffers[*output],
                        ],
                    );
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.mul_pl,
                        &bg,
                        size.div_ceil(self.workgroup_size),
                    );
                }
                SessionOp::Fma {
                    input_a,
                    input_b,
                    input_c,
                    output,
                } => {
                    let size = self.elem_count(*output) as u32;
                    let bg = self.auto_bind_group(
                        &self.pipelines.fma_pl,
                        &[
                            &self.buffers[*input_a],
                            &self.buffers[*input_b],
                            &self.buffers[*input_c],
                            &self.buffers[*output],
                        ],
                    );
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.fma_pl,
                        &bg,
                        size.div_ceil(self.workgroup_size),
                    );
                }
                SessionOp::Scale {
                    input,
                    scalar,
                    output,
                } => {
                    let size = self.elem_count(*output) as u32;
                    let params_buf =
                        self.uniform_buf(bytes_of(&ScaleParams { scalar: *scalar }), "Scale");
                    let bg = self.auto_bind_group(
                        &self.pipelines.scale_pl,
                        &[&self.buffers[*input], &params_buf, &self.buffers[*output]],
                    );
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.scale_pl,
                        &bg,
                        size.div_ceil(self.workgroup_size),
                    );
                }

                // ── Matrix multiply ───────────────────────────────────────────
                SessionOp::MatMul {
                    input_a,
                    input_b,
                    output,
                    m,
                    k,
                    n,
                    tier,
                } => {
                    let pipeline = match tier {
                        MatMulTier::Naive => &self.pipelines.mm_naive_pl,
                        MatMulTier::Tiled16 => &self.pipelines.mm_t16_pl,
                        MatMulTier::CpuTiled32 => &self.pipelines.mm_cpu_pl,
                        MatMulTier::GpuEvolved32 => &self.pipelines.mm_gpu_pl,
                    };
                    let params_buf = self.uniform_buf(
                        bytes_of(&MatMulParams {
                            m: *m,
                            k: *k,
                            n: *n,
                            _padding: 0,
                        }),
                        "MatMul",
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
                    let mut pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.dispatch_workgroups(wg_x, wg_y, 1);
                }

                // ── Activations ───────────────────────────────────────────────
                SessionOp::ReLU { input, output } => {
                    let size = self.elem_count(*output) as u32;
                    let bg = self.auto_bind_group(
                        &self.pipelines.relu_pl,
                        &[&self.buffers[*input], &self.buffers[*output]],
                    );
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.relu_pl,
                        &bg,
                        size.div_ceil(256),
                    );
                }
                SessionOp::Gelu { input, output } => {
                    let size = self.elem_count(*output) as u32;
                    let size_buf = self.uniform_u32(size, "GELU Size");
                    let bg = self.auto_bind_group(
                        &self.pipelines.gelu_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &size_buf],
                    );
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.gelu_pl,
                        &bg,
                        size.div_ceil(256),
                    );
                }
                SessionOp::Softmax { input, output } => {
                    let size = self.elem_count(*output) as u32;
                    let params_buf = self.uniform_u32(size, "Softmax Params");
                    let bg = self.auto_bind_group(
                        &self.pipelines.sfmx_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &params_buf],
                    );
                    self.dispatch_simple(&mut encoder, &self.pipelines.sfmx_pl, &bg, 1);
                }

                // ── Layer normalisation ───────────────────────────────────────
                SessionOp::LayerNorm {
                    input,
                    output,
                    feature_size,
                } => {
                    let total = self.elem_count(*output) as u32;
                    let params_buf = self.uniform_buf(
                        bytes_of(&LayerNormParams {
                            size: total,
                            feature_size: *feature_size,
                            epsilon: 1e-5,
                        }),
                        "LayerNorm",
                    );
                    let bg = self.auto_bind_group(
                        &self.pipelines.lnrm_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &params_buf],
                    );
                    let rows = (total / feature_size).max(1);
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.lnrm_pl,
                        &bg,
                        rows.div_ceil(256),
                    );
                }

                // ── Attention (3-pass SDPA) ───────────────────────────────────
                SessionOp::Attention {
                    q,
                    k,
                    v,
                    output,
                    batch_size,
                    num_heads,
                    seq_len,
                    head_dim,
                } => {
                    let attn_buf = self.uniform_buf(
                        bytes_of(&AttentionParams {
                            batch_size: *batch_size,
                            num_heads: *num_heads,
                            seq_len: *seq_len,
                            head_dim: *head_dim,
                        }),
                        "Attn",
                    );
                    let ss_bytes = (*batch_size * *num_heads * *seq_len * *seq_len) as u64
                        * std::mem::size_of::<f32>() as u64;
                    let scores_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Attn Scores"),
                        size: ss_bytes.max(4),
                        usage: wgpu::BufferUsages::STORAGE,
                        mapped_at_creation: false,
                    });
                    let weights_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Attn Weights"),
                        size: ss_bytes.max(4),
                        usage: wgpu::BufferUsages::STORAGE,
                        mapped_at_creation: false,
                    });

                    let bg1 = self.auto_bind_group(
                        &self.pipelines.sdpa_scores_pl,
                        &[&self.buffers[*q], &self.buffers[*k], &scores_buf, &attn_buf],
                    );
                    let total1 = *batch_size * *num_heads * *seq_len * *seq_len;
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.sdpa_scores_pl,
                        &bg1,
                        total1.div_ceil(256),
                    );

                    let bg2 = self.auto_bind_group(
                        &self.pipelines.attn_softmax_pl,
                        &[&scores_buf, &weights_buf, &attn_buf],
                    );
                    let total2 = *batch_size * *num_heads * *seq_len;
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.attn_softmax_pl,
                        &bg2,
                        total2.div_ceil(256),
                    );

                    let bg3 = self.auto_bind_group(
                        &self.pipelines.attn_apply_pl,
                        &[
                            &weights_buf,
                            &self.buffers[*v],
                            &self.buffers[*output],
                            &attn_buf,
                        ],
                    );
                    {
                        let mut pass =
                            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                        pass.set_pipeline(&self.pipelines.attn_apply_pl);
                        pass.set_bind_group(0, &bg3, &[]);
                        pass.dispatch_workgroups(
                            head_dim.div_ceil(16),
                            seq_len.div_ceil(16),
                            *batch_size * *num_heads,
                        );
                    }
                }

                // ── Head reshape ──────────────────────────────────────────────
                SessionOp::HeadSplit {
                    input,
                    output,
                    batch_size,
                    seq_len,
                    num_heads,
                    head_dim,
                } => {
                    let p = self.uniform_buf(
                        bytes_of(&HeadReshapeParams {
                            batch_size: *batch_size,
                            seq_len: *seq_len,
                            num_heads: *num_heads,
                            head_dim: *head_dim,
                        }),
                        "HeadSplit",
                    );
                    let bg = self.auto_bind_group(
                        &self.pipelines.head_split_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &p],
                    );
                    let total = *batch_size * *num_heads * *seq_len * *head_dim;
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.head_split_pl,
                        &bg,
                        total.div_ceil(256),
                    );
                }
                SessionOp::HeadConcat {
                    input,
                    output,
                    batch_size,
                    seq_len,
                    num_heads,
                    head_dim,
                } => {
                    let p = self.uniform_buf(
                        bytes_of(&HeadReshapeParams {
                            batch_size: *batch_size,
                            seq_len: *seq_len,
                            num_heads: *num_heads,
                            head_dim: *head_dim,
                        }),
                        "HeadConcat",
                    );
                    let bg = self.auto_bind_group(
                        &self.pipelines.head_concat_pl,
                        &[&self.buffers[*input], &self.buffers[*output], &p],
                    );
                    let total = *batch_size * *num_heads * *seq_len * *head_dim;
                    self.dispatch_simple(
                        &mut encoder,
                        &self.pipelines.head_concat_pl,
                        &bg,
                        total.div_ceil(256),
                    );
                }
            }
        }

        self.device.queue.submit(Some(encoder.finish()));
        self.device.device.poll(wgpu::Maintain::Wait);
        self.executed = true;
        Ok(())
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Allocate output buffer from a shape slice (avoids clone at call site)
    fn alloc_output(&mut self, shape: &[usize]) -> usize {
        let n = shape.iter().product::<usize>();
        let buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Session Output"),
            size: (n * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        self.register_buffer(Arc::new(buf), shape)
    }

    fn register_buffer(&mut self, buf: Arc<wgpu::Buffer>, shape: &[usize]) -> usize {
        let id = self.buffers.len();
        self.buffers.push(buf);
        self.shapes.push(shape.to_vec());
        id
    }

    /// Create a SessionTensor referencing the buffer at `id` with the given shape
    fn make_tensor(&self, id: usize, shape: &[usize]) -> SessionTensor {
        SessionTensor {
            buffer_id: id,
            shape: shape.to_vec(),
            device: self.device.clone(),
            buffer: Some(self.buffers[id].clone()),
        }
    }

    fn elem_count(&self, id: usize) -> usize {
        self.shapes[id].iter().product()
    }

    fn check_same_shape(&self, a: &SessionTensor, b: &SessionTensor) -> Result<()> {
        if a.shape() != b.shape() {
            Err(BarracudaError::shape_mismatch(
                a.shape().to_vec(),
                b.shape().to_vec(),
            ))
        } else {
            Ok(())
        }
    }

    /// Bind group derived from the pipeline's auto-reflected layout at group 0.
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
        self.device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &entries,
            })
    }

    /// Dispatch a pipeline with a single 1-D workgroup count.
    fn dispatch_simple(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        bg: &wgpu::BindGroup,
        workgroups: u32,
    ) {
        let wg = workgroups.clamp(1, 65535);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.dispatch_workgroups(wg, 1, 1);
    }

    /// Create a small uniform buffer containing raw bytes.
    fn uniform_buf(&self, bytes: &[u8], label: &str) -> wgpu::Buffer {
        self.device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            })
    }

    /// Create a uniform buffer containing a single `u32`.
    fn uniform_u32(&self, value: u32, label: &str) -> wgpu::Buffer {
        self.uniform_buf(bytemuck::bytes_of(&value), label)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool;

    #[tokio::test]
    async fn test_session_basic() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut s = TensorSession::new(&device);
        let a = s.tensor(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        let b = s.tensor(&[5.0, 6.0, 7.0, 8.0]).unwrap();
        let c = s.add(&a, &b).unwrap();
        assert_eq!(s.num_ops(), 1);
        s.run().unwrap();
        assert_eq!(c.to_vec().unwrap(), vec![6.0, 8.0, 10.0, 12.0]);
    }

    #[tokio::test]
    async fn test_session_chain() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut s = TensorSession::new(&device);
        let a = s.tensor(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        let b = s.tensor(&[2.0, 2.0, 2.0, 2.0]).unwrap();
        let c = s.add(&a, &b).unwrap();
        let d = s.mul(&c, &b).unwrap();
        assert_eq!(s.num_ops(), 2);
        s.run().unwrap();
        assert_eq!(d.to_vec().unwrap(), vec![6.0, 8.0, 10.0, 12.0]);
    }

    #[tokio::test]
    async fn test_session_fma() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut s = TensorSession::new(&device);
        let a = s.tensor(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        let b = s.tensor(&[2.0, 2.0, 2.0, 2.0]).unwrap();
        let c = s.tensor(&[10.0, 10.0, 10.0, 10.0]).unwrap();
        let d = s.fma(&a, &b, &c).unwrap();
        s.run().unwrap();
        assert_eq!(d.to_vec().unwrap(), vec![12.0, 14.0, 16.0, 18.0]);
    }

    #[tokio::test]
    async fn test_session_matmul_2x2() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut s = TensorSession::new(&device);
        let a = s.tensor_with_shape(&[1.0, 2.0, 3.0, 4.0], &[2, 2]).unwrap();
        let b = s.tensor_with_shape(&[5.0, 6.0, 7.0, 8.0], &[2, 2]).unwrap();
        let c = s.matmul(&a, &b).unwrap();
        assert_eq!(c.shape(), &[2, 2]);
        s.run().unwrap();
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
        let mut s = TensorSession::new(&device);
        let a = s.tensor(&[-1.0, 0.0, 1.0, 2.0]).unwrap();
        let b = s.relu(&a).unwrap();
        s.run().unwrap();
        let result = b.to_vec().unwrap();
        assert_eq!(result, vec![0.0, 0.0, 1.0, 2.0]);
    }

    #[tokio::test]
    async fn test_session_reset_and_reuse() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };
        let mut s = TensorSession::new(&device);
        let a = s.tensor(&[1.0, 2.0]).unwrap();
        let b = s.tensor(&[3.0, 4.0]).unwrap();
        let c = s.add(&a, &b).unwrap();
        s.run().unwrap();
        assert_eq!(c.to_vec().unwrap(), vec![4.0, 6.0]);
        // Reset and reuse the session — pipelines remain compiled
        s.reset();
        assert_eq!(s.num_ops(), 0);
    }
}
