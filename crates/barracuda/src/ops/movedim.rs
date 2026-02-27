//! MoveDim - Complete dimension reordering
//!
//! **Deep Debt Principles**:
//! - Complete implementation: Full dimension reordering, not simplified copy
//! - Zero hardcoding: All parameters configurable
//! - Self-knowledge: Validates input shapes and dimension indices
//! - Modern idiomatic Rust: Result<T, E>, pattern matching
//! - Pure GPU: No CPU fallbacks

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MoveDimParams {
    total_size: u32,
    num_dims: u32,
    source_dim: u32,
    dest_dim: u32,
    _padding: [u32; 4],
}

pub struct MoveDim {
    input: Tensor,
    source_dim: usize,
    dest_dim: usize,
}

impl MoveDim {
    pub fn new(input: Tensor, source_dim: usize, dest_dim: usize) -> Result<Self> {
        let shape = input.shape();
        let num_dims = shape.len();

        if num_dims == 0 {
            return Err(BarracudaError::invalid_op(
                "movedim",
                "Cannot move dimension of scalar tensor",
            ));
        }

        if source_dim >= num_dims {
            return Err(BarracudaError::invalid_op(
                "movedim",
                format!("source_dim {} exceeds tensor rank {}", source_dim, num_dims),
            ));
        }

        if dest_dim >= num_dims {
            return Err(BarracudaError::invalid_op(
                "movedim",
                format!("dest_dim {} exceeds tensor rank {}", dest_dim, num_dims),
            ));
        }

        Ok(Self {
            input,
            source_dim,
            dest_dim,
        })
    }

    fn wgsl_shader() -> &'static str {
        {
            static S: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
                crate::shaders::precision::downcast_f64_to_f32_with_transcendentals(include_str!(
                    "../shaders/tensor/movedim_f64.wgsl"
                ))
            });
            &S
        }
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let num_dims = shape.len();
        let total_size: usize = shape.iter().product();

        // Compute output shape (reordered)
        let mut output_shape = shape.to_vec();
        let dim_value = output_shape.remove(self.source_dim);
        output_shape.insert(self.dest_dim.min(num_dims - 1), dim_value);

        // Compute strides for input and output
        let mut input_strides = vec![1u32; num_dims];
        let mut output_strides = vec![1u32; num_dims];

        for i in (0..num_dims - 1).rev() {
            input_strides[i] = input_strides[i + 1] * shape[i + 1] as u32;
        }

        for i in (0..num_dims - 1).rev() {
            output_strides[i] = output_strides[i + 1] * output_shape[i + 1] as u32;
        }

        // Create dimension mapping
        let mut input_dims: Vec<usize> = (0..num_dims).collect();
        let moved_dim = input_dims.remove(self.source_dim);
        input_dims.insert(self.dest_dim.min(num_dims - 1), moved_dim);

        // Simplified: direct mapping
        let mut dim_mapping = vec![0u32; num_dims];
        for i in 0..num_dims {
            dim_mapping[i] = input_dims[i] as u32;
        }

        // Create buffers
        let output_buffer = device.create_buffer_f32(total_size)?;

        let input_shape_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MoveDim Input Shape"),
                    contents: bytemuck::cast_slice(
                        &shape.iter().map(|&s| s as u32).collect::<Vec<_>>(),
                    ),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let output_shape_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MoveDim Output Shape"),
                    contents: bytemuck::cast_slice(
                        &output_shape.iter().map(|&s| s as u32).collect::<Vec<_>>(),
                    ),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let input_strides_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MoveDim Input Strides"),
                    contents: bytemuck::cast_slice(&input_strides),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let output_strides_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MoveDim Output Strides"),
                    contents: bytemuck::cast_slice(&output_strides),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let dim_mapping_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MoveDim Dim Mapping"),
                    contents: bytemuck::cast_slice(&dim_mapping),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let params = MoveDimParams {
            total_size: total_size as u32,
            num_dims: num_dims as u32,
            source_dim: self.source_dim as u32,
            dest_dim: self.dest_dim as u32,
            _padding: [0; 4],
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MoveDim Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MoveDim Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MoveDim Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: input_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: input_strides_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: output_strides_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: dim_mapping_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("MoveDim"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("MoveDim Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MoveDim Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MoveDim Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MoveDim Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (total_size as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.submit_and_poll(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_movedim_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            vec![2, 3],
            device.clone(),
        )
        .await
        .unwrap();

        let result = MoveDim::new(input, 0, 1).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &[3, 2]);
    }

    #[tokio::test]
    async fn test_movedim_3d() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(
            (0..24).map(|i| i as f32).collect(),
            vec![2, 3, 4],
            device.clone(),
        )
        .await
        .unwrap();

        let result = MoveDim::new(input, 1, 2).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &[2, 4, 3]);
    }

    #[tokio::test]
    async fn test_movedim_same_dim() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2], device.clone())
            .await
            .unwrap();

        let result = MoveDim::new(input, 0, 0).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &[2, 2]);
    }

    #[tokio::test]
    async fn test_movedim_invalid_dim() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();

        assert!(MoveDim::new(input.clone(), 5, 0).is_err());
        assert!(MoveDim::new(input, 0, 5).is_err());
    }

    #[tokio::test]
    async fn test_movedim_4d() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(
            (0..120).map(|i| i as f32).collect(),
            vec![2, 3, 4, 5],
            device.clone(),
        )
        .await
        .unwrap();

        let result = MoveDim::new(input, 0, 3).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &[3, 4, 5, 2]);
    }
}
