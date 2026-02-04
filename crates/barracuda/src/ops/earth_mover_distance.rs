//! Earth Mover's Distance operation (Wasserstein-1)
//!
//! Measures distance between probability distributions
//! Also known as Wasserstein distance

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct EarthMoverDistanceParams {
    size: u32,
    _padding: [u32; 3],
}

/// Earth Mover's Distance operation
pub struct EarthMoverDistance {
    dist1: Tensor,
    dist2: Tensor,
}

impl EarthMoverDistance {
    /// Create Earth Mover's Distance operation
    pub fn new(dist1: Tensor, dist2: Tensor) -> Result<Self> {
        if dist1.shape() != dist2.shape() {
            return Err(BarracudaError::invalid_op(
                "earth_mover_distance",
                format!(
                    "dist1 shape {:?} must match dist2 shape {:?}",
                    dist1.shape(),
                    dist2.shape()
                ),
            ));
        }

        Ok(Self { dist1, dist2 })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/earth_mover_distance.wgsl")
    }

    /// Execute Earth Mover's Distance on tensors
    pub fn execute(self) -> Result<Tensor> {
        let device = self.dist1.device();
        let size = self.dist1.len();

        // Create output buffer (scalar distance)
        let output_buffer = device.create_buffer_f32(1)?;

        // Create params
        let params = EarthMoverDistanceParams {
            size: size as u32,
            _padding: [0; 3],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("EarthMoverDistance Params"),
            size: std::mem::size_of::<EarthMoverDistanceParams>() as u64,
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
                    label: Some("EarthMoverDistance Bind Group Layout"),
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
            label: Some("EarthMoverDistance Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.dist1.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.dist2.buffer().as_entire_binding(),
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
        let shader = device.compile_shader(Self::wgsl_shader(), Some("EarthMoverDistance"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("EarthMoverDistance Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("EarthMoverDistance Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("EarthMoverDistance Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("EarthMoverDistance Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (256 threads per workgroup)
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor (scalar)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![1],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_earth_mover_distance_basic() {
        let device = get_test_device().await;

        let dist1 = Tensor::from_vec_on(vec![0.5, 0.3, 0.2], vec![3], device.clone())
            .await
            .unwrap();

        let dist2 = Tensor::from_vec_on(vec![0.4, 0.4, 0.2], vec![3], device)
            .await
            .unwrap();

        let output = EarthMoverDistance::new(dist1, dist2)
            .unwrap()
            .execute()
            .unwrap();
        let result = output.to_vec().unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0] >= 0.0);
    }
}
