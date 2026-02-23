//! GPU per-link kinetic energy from HMC momenta.

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

use super::su3::su3_preamble;

const WG: u32 = 64;
const SHADER_BODY: &str = include_str!("../../shaders/lattice/kinetic_energy_f64.wgsl");

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct KineticParams {
    n_links: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// GPU kinetic energy operator: -0.5 × Re Tr(π²) per link.
pub struct GpuKineticEnergy {
    device: Arc<WgpuDevice>,
    n_links: u32,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params: wgpu::Buffer,
}

impl GpuKineticEnergy {
    pub fn new(device: Arc<WgpuDevice>, volume: u32) -> Result<Self> {
        let n_links = volume * 4;
        let src = format!("{}{}", su3_preamble(), SHADER_BODY);
        let module = device.compile_shader_f64(&src, Some("kinetic_energy"));

        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GpuKineticEnergy:bgl"),
                entries: &[
                    uniform_bgl(0),
                    storage_bgl(1, true),  // momenta
                    storage_bgl(2, false), // energy
                ],
            });

        let layout = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GpuKineticEnergy:layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("GpuKineticEnergy:pipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: "kinetic_energy_kernel",
                compilation_options: Default::default(),
                cache: None,
            });

        let params_data = KineticParams {
            n_links,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        let params = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuKineticEnergy:params"),
            size: std::mem::size_of::<KineticParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params, 0, bytemuck::bytes_of(&params_data));

        Ok(Self {
            device,
            n_links,
            pipeline,
            bgl,
            params,
        })
    }

    /// Compute per-link kinetic energy.
    ///
    /// * `momenta_buf` — `[V × 4 × 18]` f64 (conjugate momenta)
    /// * `energy_buf`  — `[V × 4]` f64 (per-link kinetic energy)
    pub fn compute(&self, momenta_buf: &wgpu::Buffer, energy_buf: &wgpu::Buffer) -> Result<()> {
        let bg = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GpuKineticEnergy:bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: momenta_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: energy_buf.as_entire_binding(),
                    },
                ],
            });

        let mut enc = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GpuKineticEnergy:enc"),
            });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GpuKineticEnergy:pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(self.n_links.div_ceil(WG), 1, 1);
        }
        self.device.queue.submit(Some(enc.finish()));
        Ok(())
    }

    pub fn n_links(&self) -> u32 {
        self.n_links
    }
}

fn storage_bgl(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_bgl(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinetic_energy_pipeline_creation() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
        else {
            return;
        };
        let op = GpuKineticEnergy::new(device, 16).unwrap();
        assert_eq!(op.n_links(), 64);
    }
}
