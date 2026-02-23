//! BESSEL J1 F64 - Bessel function of first kind, order 1 - f64 precision WGSL
//!
//! Deep Debt Principles apply. See bessel_j0_f64_wgsl.rs for details.
//!
//! Applications: Electromagnetic wave propagation, antenna patterns

use crate::device::capabilities::WORKGROUP_SIZE_1D;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// f64 Bessel J1 function evaluator
pub struct BesselJ1F64 {
    device: Arc<WgpuDevice>,
}

impl BesselJ1F64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/bessel_j1_f64.wgsl")
    }

    /// Compute J₁(x) for each element
    pub fn j1(&self, x: &[f64]) -> Result<Vec<f64>> {
        if x.is_empty() {
            return Ok(vec![]);
        }

        self.j1_gpu(x)
    }

    #[cfg(test)]
    fn j1_cpu(&self, x: &[f64]) -> Vec<f64> {
        x.iter().map(|&xi| Self::j1_scalar(xi)).collect()
    }

    #[cfg(test)]
    fn j1_scalar(x: f64) -> f64 {
        let ax = x.abs();
        if ax >= 8.0 {
            let z = 8.0 / ax;
            let z2 = z * z;
            let z4 = z2 * z2;
            let z6 = z4 * z2;
            let z8 = z4 * z4;

            let p1 = 1.0 + 1.83105e-3 * z2 - 3.516396496e-4 * z4 + 2.457520174e-5 * z6
                - 2.40337019e-6 * z8;

            let q1 = 4.687499995e-2 * z - 2.002690873e-4 * z * z2 + 8.449199096e-6 * z * z4
                - 8.8228987e-7 * z * z6
                + 1.057874120e-7 * z * z8;

            let sqrt_2_over_pi = 0.7978845608028654;
            let three_pi_over_4 = 2.356_194_490_192_345;
            let inv_sqrt_x = sqrt_2_over_pi / ax.sqrt();
            let xx = ax - three_pi_over_4;
            let r = inv_sqrt_x * (p1 * xx.cos() - q1 * xx.sin());
            if x < 0.0 {
                -r
            } else {
                r
            }
        } else {
            let z = x * x;
            let z2 = z * z;
            let z3 = z2 * z;
            let z4 = z2 * z2;
            let z5 = z2 * z3;

            let p = 72362614232.0 - 7895059235.0 * z + 242396853.1 * z2 - 2972611.439 * z3
                + 15704.48260 * z4
                - 30.16036606 * z5;

            let q = 144725228442.0
                + 2300535178.0 * z
                + 18583304.74 * z2
                + 99447.43394 * z3
                + 376.9991397 * z4
                + z5;

            x * (p / q)
        }
    }

    fn j1_gpu(&self, x: &[f64]) -> Result<Vec<f64>> {
        let size = x.len();

        let input_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Bessel J1 f64 Input"),
                contents: bytemuck::cast_slice(x),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bessel J1 f64 Output"),
            size: std::mem::size_of_val(x) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Metadata {
            size: u32,
            _pad0: u32,
            _pad1: u32,
            _pad2: u32,
        }

        let metadata = Metadata {
            size: size as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        let metadata_buf =
            self.device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Bessel J1 f64 Metadata"),
                    contents: bytemuck::bytes_of(&metadata),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let shader_module = self
            .device
            .compile_shader_f64(Self::wgsl_shader(), Some("Bessel J1 f64"));

        let bind_group_layout =
            self.device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Bessel J1 f64 BGL"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bessel J1 f64 BG"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: metadata_buf.as_entire_binding(),
                    },
                ],
            });

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Bessel J1 f64 PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Bessel J1 f64 Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Bessel J1 f64 Encoder"),
                });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Bessel J1 f64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32).div_ceil(WORKGROUP_SIZE_1D), 1, 1);
        }

        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bessel J1 f64 Staging"),
            size: std::mem::size_of_val(x) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(
            &output_buf,
            0,
            &staging_buf,
            0,
            std::mem::size_of_val(x) as u64,
        );
        self.device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| BarracudaError::Device(format!("Channel error: {}", e)))?
            .map_err(|e| BarracudaError::Device(format!("Buffer map error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> Option<Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
    }

    #[test]
    fn test_j1_at_zero() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let bessel = BesselJ1F64::new(device)?;
        let result = bessel.j1(&[0.0])?;
        assert!(
            (result[0]).abs() < 1e-10,
            "J₁(0) = {}, expected 0",
            result[0]
        );
        Ok(())
    }

    #[test]
    fn test_j1_known_values() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let bessel = BesselJ1F64::new(device)?;
        let x = vec![1.0, 2.0, 5.0];
        let expected = vec![0.4400505857449335, 0.5767248077568734, -0.3275791375914652];
        let result = bessel.j1(&x)?;
        for (i, &val) in result.iter().enumerate() {
            assert!(
                (val - expected[i]).abs() < 1e-6,
                "J₁({}) = {}, expected {}",
                x[i],
                val,
                expected[i]
            );
        }
        Ok(())
    }

    #[test]
    fn test_j1_antisymmetry() -> Result<()> {
        let Some(device) = create_test_device() else {
            return Ok(());
        };
        let bessel = BesselJ1F64::new(device)?;
        let x = vec![-2.0, 2.0];
        let result = bessel.j1(&x)?;
        assert!((result[0] + result[1]).abs() < 1e-10, "J₁(-x) != -J₁(x)");
        Ok(())
    }
}
