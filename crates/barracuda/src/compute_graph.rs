//! Compute Graph - Lazy Execution for Operation Batching
//!
//! **Problem**: wgpu has significant per-dispatch overhead (~50-100μs).
//! Individual operations like `tensor.add(&b)` each submit a separate command buffer.
//!
//! **Solution**: Record operations lazily, then batch them into a single submission.
//!
//! **Usage**:
//! ```rust,no_run
//! # use barracuda::prelude::*;
//! # async fn example() -> Result<()> {
//! let device = WgpuDevice::new().await?;
//! let mut graph = ComputeGraph::new(&device);
//!
//! // Record operations (no GPU work yet)
//! let a = Tensor::<f32>::from_slice(&device, &[1.0, 2.0, 3.0])?;
//! let b = Tensor::<f32>::from_slice(&device, &[4.0, 5.0, 6.0])?;
//! let c = graph.add(&a, &b)?;  // Just records
//! let d = graph.mul(&c, &b)?;  // Just records
//!
//! // Execute all operations in one batch
//! graph.execute()?;
//!
//! // Now read results
//! let result = d.to_vec()?;
//! # Ok(())
//! # }
//! ```
//!
//! **Performance Impact**: Batching 100 operations reduces overhead from
//! ~10ms (100 × 100μs) to ~200μs (1 × 100μs + 100 × 1μs kernel dispatch).

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// Recorded operation in the compute graph
#[derive(Debug)]
pub enum RecordedOp {
    /// Element-wise add: output = a + b
    Add {
        input_a: wgpu::Buffer,
        input_b: wgpu::Buffer,
        output: wgpu::Buffer,
        size: usize,
    },
    /// Element-wise multiply: output = a * b
    Mul {
        input_a: wgpu::Buffer,
        input_b: wgpu::Buffer,
        output: wgpu::Buffer,
        size: usize,
    },
    /// Fused multiply-add: output = a * b + c
    Fma {
        input_a: wgpu::Buffer,
        input_b: wgpu::Buffer,
        input_c: wgpu::Buffer,
        output: wgpu::Buffer,
        size: usize,
    },
    /// Scale: output = a * scalar
    Scale {
        input: wgpu::Buffer,
        scalar: f32,
        output: wgpu::Buffer,
        size: usize,
    },
    /// Custom shader operation
    Custom {
        shader_source: String,
        buffers: Vec<wgpu::Buffer>,
        workgroups: (u32, u32, u32),
    },
}

/// Compute graph for batching GPU operations
///
/// Records operations without executing them, then batches
/// all operations into a single command buffer submission.
pub struct ComputeGraph {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    #[allow(dead_code)] // Reserved for future debug logging
    device_name: String,
    ops: Vec<RecordedOp>,
    optimal_workgroup_size: u32,
}

impl ComputeGraph {
    /// Create a new compute graph for a device
    pub fn new(wgpu_device: &WgpuDevice) -> Self {
        let optimal_wg = wgpu_device.optimal_workgroup_size();
        Self {
            device: wgpu_device.device.clone(),
            queue: wgpu_device.queue.clone(),
            device_name: wgpu_device.name().to_string(),
            ops: Vec::new(),
            optimal_workgroup_size: optimal_wg,
        }
    }

    /// Record an add operation
    pub fn record_add(
        &mut self,
        input_a: wgpu::Buffer,
        input_b: wgpu::Buffer,
        output: wgpu::Buffer,
        size: usize,
    ) {
        self.ops.push(RecordedOp::Add {
            input_a,
            input_b,
            output,
            size,
        });
    }

    /// Record a multiply operation
    pub fn record_mul(
        &mut self,
        input_a: wgpu::Buffer,
        input_b: wgpu::Buffer,
        output: wgpu::Buffer,
        size: usize,
    ) {
        self.ops.push(RecordedOp::Mul {
            input_a,
            input_b,
            output,
            size,
        });
    }

    /// Record a fused multiply-add operation
    pub fn record_fma(
        &mut self,
        input_a: wgpu::Buffer,
        input_b: wgpu::Buffer,
        input_c: wgpu::Buffer,
        output: wgpu::Buffer,
        size: usize,
    ) {
        self.ops.push(RecordedOp::Fma {
            input_a,
            input_b,
            input_c,
            output,
            size,
        });
    }

    /// Number of recorded operations
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Clear all recorded operations
    pub fn clear(&mut self) {
        self.ops.clear();
    }

    /// Execute all recorded operations in a single batch
    ///
    /// This is where the performance magic happens - all operations
    /// are encoded into a single command buffer and submitted together.
    pub fn execute(&mut self) -> Result<()> {
        if self.ops.is_empty() {
            return Ok(());
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ComputeGraph Batch"),
            });

        // Pre-compile shaders (would be cached in production)
        let add_shader = self.compile_add_shader();
        let mul_shader = self.compile_mul_shader();
        let fma_shader = self.compile_fma_shader();
        let scale_shader = self.compile_scale_shader();

        for op in &self.ops {
            match op {
                RecordedOp::Add {
                    input_a,
                    input_b,
                    output,
                    size,
                } => {
                    self.encode_binary_op(
                        &mut encoder,
                        &add_shader,
                        input_a,
                        input_b,
                        output,
                        *size,
                    );
                }
                RecordedOp::Mul {
                    input_a,
                    input_b,
                    output,
                    size,
                } => {
                    self.encode_binary_op(
                        &mut encoder,
                        &mul_shader,
                        input_a,
                        input_b,
                        output,
                        *size,
                    );
                }
                RecordedOp::Fma {
                    input_a,
                    input_b,
                    input_c,
                    output,
                    size,
                } => {
                    self.encode_fma_op(
                        &mut encoder,
                        &fma_shader,
                        input_a,
                        input_b,
                        input_c,
                        output,
                        *size,
                    );
                }
                RecordedOp::Scale {
                    input,
                    scalar,
                    output,
                    size,
                } => {
                    self.encode_scale_op(
                        &mut encoder,
                        &scale_shader,
                        input,
                        *scalar,
                        output,
                        *size,
                    );
                }
                RecordedOp::Custom {
                    shader_source,
                    buffers,
                    workgroups,
                } => {
                    self.encode_custom_op(&mut encoder, shader_source, buffers, *workgroups);
                }
            }
        }

        // Single submission for all operations
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);

        // Clear recorded ops
        self.ops.clear();

        Ok(())
    }

    fn compile_add_shader(&self) -> wgpu::ShaderModule {
        let source = format!(
            r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{
        return;
    }}
    output[idx] = a[idx] + b[idx];
}}
"#,
            self.optimal_workgroup_size
        );

        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Add Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    fn compile_mul_shader(&self) -> wgpu::ShaderModule {
        let source = format!(
            r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{
        return;
    }}
    output[idx] = a[idx] * b[idx];
}}
"#,
            self.optimal_workgroup_size
        );

        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Mul Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    fn compile_fma_shader(&self) -> wgpu::ShaderModule {
        let source = format!(
            r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read> c: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{
        return;
    }}
    output[idx] = fma(a[idx], b[idx], c[idx]);
}}
"#,
            self.optimal_workgroup_size
        );

        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FMA Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    fn compile_scale_shader(&self) -> wgpu::ShaderModule {
        let source = format!(
            r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<uniform> scalar: f32;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({wg})
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let idx = gid.x;
    if (idx >= arrayLength(&output)) {{
        return;
    }}
    output[idx] = input[idx] * scalar;
}}
"#,
            wg = self.optimal_workgroup_size
        );
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Scale Shader"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let workgroups = (size as u32)
            .div_ceil(self.optimal_workgroup_size)
            .min(65535);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }

    fn encode_fma_op(
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let workgroups = (size as u32)
            .div_ceil(self.optimal_workgroup_size)
            .min(65535);

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
        let scalar_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scalar Uniform"),
            size: 4, // f32
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&scalar_buffer, 0, bytemuck::bytes_of(&scalar));

        let bgl = self
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let workgroups = (size as u32)
            .div_ceil(self.optimal_workgroup_size)
            .min(65535);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }

    fn encode_custom_op(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shader_source: &str,
        buffers: &[wgpu::Buffer],
        workgroups: (u32, u32, u32),
    ) {
        // Compile custom shader
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Custom Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Build bind group layout dynamically based on buffer count
        // Assumes first N-1 buffers are read-only, last is read-write
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let read_only = i < buffers.len() - 1;
                wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            })
            .collect();

        let bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &entries,
            });

        let bind_entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, buf)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buf.as_entire_binding(),
            })
            .collect();

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &bind_entries,
        });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool;
    use wgpu::util::DeviceExt;

    #[tokio::test]
    async fn test_compute_graph_batching() {
        let Some(device) = test_pool::get_test_device_if_gpu_available().await else {
            return;
        };

        let mut graph = ComputeGraph::new(&device);

        // Create test data
        let data_a: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let data_b: Vec<f32> = (0..1000).map(|i| (i * 2) as f32).collect();

        // Create buffers
        let buf_a = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("A"),
                contents: bytemuck::cast_slice(&data_a),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        let buf_b = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("B"),
                contents: bytemuck::cast_slice(&data_b),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        let buf_out = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Out"),
            size: (1000 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Record operation
        graph.record_add(buf_a, buf_b, buf_out, 1000);

        assert_eq!(graph.len(), 1);

        // Execute
        graph.execute().unwrap();

        assert!(graph.is_empty());
    }
}
