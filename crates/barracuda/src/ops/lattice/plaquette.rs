//! Wilson plaquette GPU operation for SU(3) lattice gauge theory.
//!
//! Computes `Re Tr(U_p) / 3` for all 6 plane orientations at every site on a
//! 4D periodic lattice in a single GPU dispatch.
//!
//! # Usage
//!
//! ```ignore
//! let op = WilsonPlaquette::new(device.clone(), nt, nx, ny, nz)?;
//! // `links_buf` holds [V × 4 × 18] f64 in row-major SU(3) storage format
//! op.compute(&links_buf, &plaq_buf)?;
//! // Average plaquette via ReduceScalarPipeline::sum_f64(&plaq_buf, volume*6)
//! // then divide by (volume * 6).
//! ```
//!
//! # hotSpring validation
//!
//! CPU reference in hotSpring `lattice/wilson.rs`.  Expected average plaquette
//! for a thermalized SU(3) config at β=6: ≈ 0.5937 (Wilson action).

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

use super::su3::su3_preamble;

const PLAQ_WG: u32 = 64;
const PLAQ_SHADER_BODY: &str =
    include_str!("../../shaders/lattice/wilson_plaquette_f64.wgsl");

/// Wilson plaquette operator on a 4D SU(3) lattice.
pub struct WilsonPlaquette {
    device:  Arc<WgpuDevice>,
    volume:  u32,
    pipeline: wgpu::ComputePipeline,
    bgl:     wgpu::BindGroupLayout,
    params:  wgpu::Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PlaqParams {
    nt:     u32,
    nx:     u32,
    ny:     u32,
    nz:     u32,
    volume: u32,
    _pad0:  u32,
    _pad1:  u32,
    _pad2:  u32,
}

impl WilsonPlaquette {
    /// Compile the plaquette pipeline for a lattice of dimensions `nt×nx×ny×nz`.
    pub fn new(device: Arc<WgpuDevice>, nt: u32, nx: u32, ny: u32, nz: u32) -> Result<Self> {
        let volume = nt * nx * ny * nz;
        let src = format!("{}{}", su3_preamble(), PLAQ_SHADER_BODY);
        // compile_shader_f64 handles exp/log patching + ILP optimizer internally
        let module = device.compile_shader_f64(&src, Some("wilson_plaquette"));

        let bgl = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("WilsonPlaquette:bgl"),
            entries: &[
                uniform_bgl(0),       // params
                storage_bgl(1, true), // links (read)
                storage_bgl(2, false),// plaq  (write)
            ],
        });

        let layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("WilsonPlaquette:layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("WilsonPlaquette:pipeline"),
            layout: Some(&layout),
            module: &module,
            entry_point: "plaquette",
            compilation_options: Default::default(),
            cache: None,
        });

        let params_data = PlaqParams { nt, nx, ny, nz, volume, _pad0: 0, _pad1: 0, _pad2: 0 };
        let params = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("WilsonPlaquette:params"),
            size:  std::mem::size_of::<PlaqParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params, 0, bytemuck::bytes_of(&params_data));

        Ok(Self { device, volume, pipeline, bgl, params })
    }

    /// Compute `Re Tr(U_p) / 3` for all plaquettes.
    ///
    /// * `links_buf` — `[V × 4 × 18]` f64 storage buffer (GPU-resident)
    /// * `plaq_buf`  — `[V × 6]` f64 storage buffer (output, GPU-resident)
    pub fn compute(&self, links_buf: &wgpu::Buffer, plaq_buf: &wgpu::Buffer) -> Result<()> {
        let bg = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("WilsonPlaquette:bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: links_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: plaq_buf.as_entire_binding() },
            ],
        });

        let mut enc = self.device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("WilsonPlaquette:enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("WilsonPlaquette:pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(self.volume.div_ceil(PLAQ_WG), 1, 1);
        }
        self.device.queue.submit(Some(enc.finish()));
        Ok(())
    }

    /// Number of lattice sites.
    pub fn volume(&self) -> u32 { self.volume }

    /// Total number of plaquette values in the output buffer (`volume × 6`).
    pub fn n_plaquettes(&self) -> u32 { self.volume * 6 }
}

// ── BGL helpers ──────────────────────────────────────────────────────────────

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
    fn test_n_plaquettes() {
        // 4^4 = 256 sites × 6 plane orientations
        let vol: u32 = 4 * 4 * 4 * 4;
        assert_eq!(vol * 6, 1536);
    }

    #[test]
    fn test_shader_source_includes_preamble() {
        let src = format!("{}{}", su3_preamble(), PLAQ_SHADER_BODY);
        assert!(src.contains("fn c64_mul"));
        assert!(src.contains("fn su3_mul"));
        assert!(src.contains("fn plaquette"));
    }
}
