//! TensorSession - Automatic Operation Batching
//!
//! **Problem**: Individual tensor operations have ~250μs overhead each.
//! A chain of 100 operations = 25ms of pure overhead!
//!
//! **Solution**: Sessions batch operations and execute together.
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
//! let a = session.tensor(&[1.0, 2.0, 3.0, 4.0])?;
//! let b = session.tensor(&[5.0, 6.0, 7.0, 8.0])?;
//! let c = session.add(&a, &b)?;  // Just records
//! let d = session.mul(&c, &b)?;  // Just records
//!
//! // Execute all operations in one batch
//! session.run()?;
//!
//! // Now read results
//! let result = d.to_vec()?;
//! # Ok(())
//! # }
//! ```
//!
//! **Performance**: Reduces overhead from N×250μs to 1×250μs + N×~1μs

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;

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

/// Recorded operation in a session
#[derive(Debug)]
enum SessionOp {
    /// Add two tensors: output = a + b
    Add {
        input_a: usize,
        input_b: usize,
        output: usize,
    },
    /// Multiply two tensors: output = a * b
    Mul {
        input_a: usize,
        input_b: usize,
        output: usize,
    },
    /// Fused multiply-add: output = a * b + c
    Fma {
        input_a: usize,
        input_b: usize,
        input_c: usize,
        output: usize,
    },
    /// Scale: output = a * scalar
    Scale {
        input: usize,
        scalar: f32,
        output: usize,
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

        // Pre-compile shaders
        let add_shader = self.compile_shader("add");
        let mul_shader = self.compile_shader("mul");
        let fma_shader = self.compile_shader("fma");
        let scale_shader = self.compile_shader("scale");

        // Create single command encoder for all operations
        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("TensorSession Batch"),
                });

        // Encode all operations
        for op in &self.ops {
            match op {
                SessionOp::Add {
                    input_a,
                    input_b,
                    output,
                } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_binary_op(
                        &mut encoder,
                        &add_shader,
                        &self.buffers[*input_a],
                        &self.buffers[*input_b],
                        &self.buffers[*output],
                        size,
                    );
                }
                SessionOp::Mul {
                    input_a,
                    input_b,
                    output,
                } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_binary_op(
                        &mut encoder,
                        &mul_shader,
                        &self.buffers[*input_a],
                        &self.buffers[*input_b],
                        &self.buffers[*output],
                        size,
                    );
                }
                SessionOp::Fma {
                    input_a,
                    input_b,
                    input_c,
                    output,
                } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_ternary_op(
                        &mut encoder,
                        &fma_shader,
                        &self.buffers[*input_a],
                        &self.buffers[*input_b],
                        &self.buffers[*input_c],
                        &self.buffers[*output],
                        size,
                    );
                }
                SessionOp::Scale {
                    input,
                    scalar,
                    output,
                } => {
                    let size = self.shapes[*output].iter().product::<usize>();
                    self.encode_scale_op(
                        &mut encoder,
                        &scale_shader,
                        &self.buffers[*input],
                        *scalar,
                        &self.buffers[*output],
                        size,
                    );
                }
            }
        }

        // Single submission for all operations
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

    fn compile_shader(&self, op_type: &str) -> wgpu::ShaderModule {
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
            _ => panic!("Unknown op type: {}", op_type),
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
}
