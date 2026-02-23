//! GPU Wilson action computation.
//!
//! Per-site action contribution dispatched on GPU; host-side reduction
//! via `ReduceScalarPipeline` yields the total Wilson action.

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

use super::su3::su3_preamble;

const WG: u32 = 64;
const SHADER_BODY: &str = include_str!("../../shaders/lattice/wilson_action_f64.wgsl");

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ActionParams {
    nt: u32,
    nx: u32,
    ny: u32,
    nz: u32,
    volume: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// GPU Wilson action operator.
pub struct GpuWilsonAction {
    device: Arc<WgpuDevice>,
    volume: u32,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params: wgpu::Buffer,
}

impl GpuWilsonAction {
    pub fn new(device: Arc<WgpuDevice>, nt: u32, nx: u32, ny: u32, nz: u32) -> Result<Self> {
        let volume = nt * nx * ny * nz;
        let src = format!("{}{}", su3_preamble(), SHADER_BODY);
        let module = device.compile_shader_f64(&src, Some("wilson_action"));

        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GpuWilsonAction:bgl"),
                entries: &[
                    uniform_bgl(0),
                    storage_bgl(1, true),  // links
                    storage_bgl(2, false), // action
                ],
            });

        let layout = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GpuWilsonAction:layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("GpuWilsonAction:pipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: "wilson_action_kernel",
                compilation_options: Default::default(),
                cache: None,
            });

        let params_data = ActionParams {
            nt,
            nx,
            ny,
            nz,
            volume,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        let params = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuWilsonAction:params"),
            size: std::mem::size_of::<ActionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params, 0, bytemuck::bytes_of(&params_data));

        Ok(Self {
            device,
            volume,
            pipeline,
            bgl,
            params,
        })
    }

    /// Compute per-site Wilson action contributions.
    ///
    /// * `links_buf`  — `[V × 4 × 18]` f64 (gauge config)
    /// * `action_buf` — `[V]` f64 (per-site output)
    ///
    /// Multiply total sum by β for the full Wilson action.
    pub fn compute(&self, links_buf: &wgpu::Buffer, action_buf: &wgpu::Buffer) -> Result<()> {
        let bg = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GpuWilsonAction:bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: links_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: action_buf.as_entire_binding(),
                    },
                ],
            });

        let mut enc = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GpuWilsonAction:enc"),
            });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GpuWilsonAction:pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(self.volume.div_ceil(WG), 1, 1);
        }
        self.device.queue.submit(Some(enc.finish()));
        Ok(())
    }

    pub fn volume(&self) -> u32 {
        self.volume
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
    fn test_wilson_action_pipeline_creation() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
        else {
            return;
        };
        let op = GpuWilsonAction::new(device, 2, 2, 2, 2).unwrap();
        assert_eq!(op.volume(), 16);
    }

    #[test]
    fn test_wilson_action_cold_start_is_zero_gpu() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
        else {
            return;
        };

        let (nt, nx, ny, nz) = (2u32, 2, 2, 2);
        let volume = (nt * nx * ny * nz) as usize;

        let identity_18: [f64; 18] = [
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            0.0,
        ];
        let links_f64: Vec<f64> = std::iter::repeat_n(identity_18.iter().copied(), volume * 4)
            .flatten()
            .collect();

        let link_bytes: &[u8] = bytemuck::cast_slice(&links_f64);
        let links_buf = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test:links"),
            size: link_bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&links_buf, 0, link_bytes);

        let action_bytes = volume * std::mem::size_of::<f64>();
        let action_buf = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test:action"),
            size: action_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let op = GpuWilsonAction::new(device.clone(), nt, nx, ny, nz).unwrap();
        op.compute(&links_buf, &action_buf).unwrap();

        let staging = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test:staging"),
            size: action_bytes as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc = device.device.create_command_encoder(&Default::default());
        enc.copy_buffer_to_buffer(&action_buf, 0, &staging, 0, action_bytes as u64);
        device.queue.submit(Some(enc.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let mapped = slice.get_mapped_range();
        let action_out: &[f64] = bytemuck::cast_slice(&mapped);

        let total: f64 = action_out.iter().sum();
        assert!(
            total.abs() < 1e-10,
            "Wilson action for cold start should be 0, got {total}"
        );
    }
}
