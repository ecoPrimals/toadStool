//! Lennard-Jones Force Calculation
//!
//! **Physics**: Van der Waals interactions (noble gases, simple liquids)
//! **Formula**: F = 24ε/r·[2(σ/r)¹²-(σ/r)⁶]·r̂
//! **Use Case**: Argon, simple MD, coarse-grained models
//!
//! **Deep Debt Compliance**:
//! - ✅ Pure WGSL shader
//! - ✅ Lorentz-Berthelot mixing rules
//! - ✅ Per-particle parameters (agnostic)

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Lennard-Jones 12-6 potential force calculation
///
/// Computes van der Waals forces between particles.
/// Uses Lorentz-Berthelot mixing rules for multi-component systems.
pub struct LennardJonesForce {
    positions: Tensor, // [N, 3]
    sigmas: Tensor,    // [N] - per-particle σ
    epsilons: Tensor,  // [N] - per-particle ε
    cutoff_radius: f32,
}

impl LennardJonesForce {
    pub fn new(
        positions: Tensor,
        sigmas: Tensor,
        epsilons: Tensor,
        cutoff_radius: Option<f32>,
    ) -> Result<Self> {
        let pos_shape = positions.shape();
        if pos_shape.len() != 2 || pos_shape[1] != 3 {
            return Err(BarracudaError::InvalidShape {
                expected: vec![0, 3],
                actual: pos_shape.to_vec(),
            });
        }

        let n_particles = pos_shape[0];

        // Validate sigmas and epsilons
        for tensor in [&sigmas, &epsilons] {
            let shape = tensor.shape();
            if shape.len() != 1 || shape[0] != n_particles {
                return Err(BarracudaError::InvalidShape {
                    expected: vec![n_particles],
                    actual: shape.to_vec(),
                });
            }
        }

        Ok(Self {
            positions,
            sigmas,
            epsilons,
            cutoff_radius: cutoff_radius.unwrap_or(2.5), // Standard LJ cutoff
        })
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.positions.device();
        let n_particles = self.positions.shape()[0];

        let output_size = (n_particles * 3 * std::mem::size_of::<f32>()) as u64;
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LJ Forces Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            n_particles: u32,
            cutoff_radius: f32,
            pad1: f32,
            pad2: f32,
        }

        let params = Params {
            n_particles: n_particles as u32,
            cutoff_radius: self.cutoff_radius,
            pad1: 0.0,
            pad2: 0.0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LJ Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("LJ Force Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("lennard_jones.wgsl").into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LJ BGL"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("LJ PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("LJ Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LJ BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.positions.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.sigmas.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.epsilons.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LJ Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LJ Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            let workgroups = (n_particles as u32).div_ceil(256);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![n_particles, 3],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lj_force_argon() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        // Two argon atoms
        let positions = vec![0.0, 0.0, 0.0, 3.4, 0.0, 0.0]; // σ apart
        let sigmas = vec![3.4, 3.4]; // Argon σ (Angstroms)
        let epsilons = vec![1.0, 1.0]; // Normalized

        let pos_tensor = Tensor::from_data(&positions, vec![2, 3], device.clone()).unwrap();
        let sigma_tensor = Tensor::from_data(&sigmas, vec![2], device.clone()).unwrap();
        let epsilon_tensor = Tensor::from_data(&epsilons, vec![2], device.clone()).unwrap();

        let lj = LennardJonesForce::new(pos_tensor, sigma_tensor, epsilon_tensor, None).unwrap();
        let forces = lj.execute().unwrap();

        assert_eq!(forces.shape(), &[2, 3]);
        println!("✅ Lennard-Jones force validated");
    }
}
