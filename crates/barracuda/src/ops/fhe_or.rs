//! FHE OR Gate Operation
//!
//! **Purpose**: Perform Boolean OR on FHE-encrypted data using GPU
//!
//! **Deep Debt Compliance**:
//! - ✅ Pure Rust + WGSL (no unsafe)
//! - ✅ Hardware-agnostic (wgpu backend selection)
//! - ✅ Numerically precise (modular arithmetic)
//! - ✅ Production-ready (full error handling)
//! - ✅ Canonical pattern: Tensor inputs/outputs, device from runtime

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// FHE OR gate operation
///
/// Performs Boolean OR on encrypted data using polynomial representation.
/// OR(a,b) = a + b - (a * b) mod q
pub struct FheOr {
    poly_a: Tensor,
    poly_b: Tensor,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    degree: u32,
    modulus: u64,
}

impl FheOr {
    /// Create a new FHE OR gate operation
    pub fn new(poly_a: Tensor, poly_b: Tensor, degree: u32, modulus: u64) -> Result<Self> {
        if poly_a.len() != degree as usize || poly_b.len() != degree as usize {
            return Err(BarracudaError::Device(format!(
                "Polynomial length mismatch: expected {}, got {} and {}",
                degree,
                poly_a.len(),
                poly_b.len()
            )));
        }

        if !std::ptr::eq(poly_a.device().as_ref(), poly_b.device().as_ref()) {
            return Err(BarracudaError::Device(
                "poly_a and poly_b must be on the same device".to_string(),
            ));
        }

        if modulus == 0 {
            return Err(BarracudaError::Device(
                "Modulus must be non-zero".to_string(),
            ));
        }

        let device = poly_a.device();

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FHE OR Gate Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("fhe_or.wgsl").into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FHE OR Bind Group Layout"),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FHE OR Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("FHE OR Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        Ok(Self {
            poly_a,
            poly_b,
            pipeline,
            bind_group_layout,
            degree,
            modulus,
        })
    }

    /// Execute the OR gate on two encrypted polynomials
    pub fn execute(self) -> Result<Tensor> {
        let device = self.poly_a.device();

        let result_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FHE OR Output"),
            size: (self.degree as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = [
            self.degree,
            (self.modulus & 0xFFFFFFFF) as u32,
            (self.modulus >> 32) as u32,
            0u32,
        ];
        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FHE OR Params"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FHE OR Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.poly_a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.poly_b.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: result_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FHE OR Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FHE OR Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::FHE);
            let workgroups = self.degree.div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.submit_and_poll(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            result_buffer,
            vec![self.degree as usize],
            device.clone(),
        ))
    }
}

/// Helper: Create FHE bit tensor from u64 coefficients (for bitwise ops)
pub async fn create_fhe_bit_tensor(
    poly: &[u64],
    device: Arc<crate::device::WgpuDevice>,
) -> Result<Tensor> {
    let poly_u32: Vec<u32> = poly.iter().map(|&x| x as u32).collect();
    Tensor::from_data_pod(&poly_u32, vec![poly_u32.len()], device)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    use crate::ops::fhe_and::create_fhe_bit_tensor;

    #[allow(unused_imports)]
    use wgpu::util::DeviceExt;

    #[tokio::test]
    async fn test_fhe_or_ones() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };
        let poly_a_data = vec![1u64; 8];
        let poly_b_data = vec![1u64; 8];

        let poly_a = create_fhe_bit_tensor(&poly_a_data, device.clone())
            .await
            .unwrap();
        let poly_b = create_fhe_bit_tensor(&poly_b_data, device.clone())
            .await
            .unwrap();

        let op = FheOr::new(poly_a, poly_b, 8, 251).unwrap();
        let result_tensor = op.execute().unwrap();

        let size = result_tensor.len();
        let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Test Staging"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            result_tensor.buffer(),
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<u32>()) as u64,
        );
        device.submit_and_poll(Some(encoder.finish()));

        let result_u32: Vec<u32> = device.map_staging_buffer(&staging_buffer, size).unwrap();

        assert_eq!(result_u32.len(), 8);
        assert!(result_u32.iter().all(|&x| x == 1), "1 OR 1 should equal 1");
    }
}
