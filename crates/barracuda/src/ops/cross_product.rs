//! Cross product operation - Vector cross product
//!
//! Computes cross product of 3D vectors
//! a × b = (a_y*b_z - a_z*b_y, a_z*b_x - a_x*b_z, a_x*b_y - a_y*b_x)

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CrossProductParams {
    num_vectors: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,
    _pad5: u32,
    _pad6: u32,
}

/// Cross product operation
pub struct CrossProduct {
    input_a: Tensor,
    input_b: Tensor,
}

impl CrossProduct {
    /// Create cross product operation
    pub fn new(input_a: Tensor, input_b: Tensor) -> Result<Self> {
        let shape_a = input_a.shape();
        let shape_b = input_b.shape();

        if shape_a.len() != 2 || shape_a[1] != 3 {
            return Err(BarracudaError::invalid_op(
                "cross_product",
                format!("input_a must be [N, 3], got shape {:?}", shape_a),
            ));
        }

        if shape_b.len() != 2 || shape_b[1] != 3 {
            return Err(BarracudaError::invalid_op(
                "cross_product",
                format!("input_b must be [N, 3], got shape {:?}", shape_b),
            ));
        }

        if shape_a[0] != shape_b[0] {
            return Err(BarracudaError::invalid_op(
                "cross_product",
                format!(
                    "input_a and input_b must have same batch size, got {} and {}",
                    shape_a[0], shape_b[0]
                ),
            ));
        }

        Ok(Self { input_a, input_b })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/misc/cross_product.wgsl")
    }

    /// Execute cross product on tensors
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input_a.device();
        let shape_a = self.input_a.shape();
        let num_vectors = shape_a[0];

        // Create output buffer [N, 3]
        let output_size = num_vectors * 3;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params
        let params = CrossProductParams {
            num_vectors: num_vectors as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
            _pad4: 0,
            _pad5: 0,
            _pad6: 0,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CrossProduct Params"),
            size: std::mem::size_of::<CrossProductParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CrossProduct Bind Group Layout"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CrossProduct Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input_a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.input_b.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("CrossProduct"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("CrossProduct Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("CrossProduct Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("CrossProduct Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("CrossProduct Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            use crate::device::{DeviceCapabilities, WorkloadType};
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (num_vectors as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![num_vectors, 3],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_cross_product_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Test vectors: [1, 0, 0] × [0, 1, 0] = [0, 0, 1]
        let input_a = Tensor::from_vec_on(vec![1.0, 0.0, 0.0], vec![1, 3], device.clone())
            .await
            .unwrap();

        let input_b = Tensor::from_vec_on(vec![0.0, 1.0, 0.0], vec![1, 3], device)
            .await
            .unwrap();

        let output = CrossProduct::new(input_a, input_b)
            .unwrap()
            .execute()
            .unwrap();
        let result = output.to_vec().unwrap();

        assert_eq!(result.len(), 3);
        // Result should be approximately [0, 0, 1]
        assert!((result[0] - 0.0).abs() < 1e-5);
        assert!((result[1] - 0.0).abs() < 1e-5);
        assert!((result[2] - 1.0).abs() < 1e-5);
    }
}
