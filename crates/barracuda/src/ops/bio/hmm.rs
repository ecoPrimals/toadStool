// SPDX-License-Identifier: AGPL-3.0-only

//! Batch HMM forward algorithm (f64, GPU).
//!
//! One thread per observation sequence. Sequential over T time steps within
//! each sequence, parallel across B sequences. All computation in log-domain
//! with `log_sum_exp2` for numerical stability.
//!
//! ## Absorbed from
//!
//! wetSpring handoff v6, `hmm_forward_f64.wgsl` — 13/13 GPU checks PASS.
//! Polyfill required for Ada Lovelace (uses f64 exp/log in `log_sum_exp2`).

use crate::device::WgpuDevice;
use crate::error::Result;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

const SHADER: &str = include_str!("../../shaders/bio/hmm_forward_f64.wgsl");

/// Log-domain f32 HMM forward shader (neuralSpring metalForge provenance).
///
/// Uses max-subtract trick for numerical stability. One thread per destination
/// state; lighter-weight than the f64 variant above, suitable for real-time
/// inference where f32 precision suffices.
pub const WGSL_HMM_FORWARD_LOG_F32: &str = include_str!("../../shaders/ml/hmm_forward_log.wgsl");

/// f64 version of the log-domain HMM forward pass for universal math library.
/// Wired and ready; no separate log-domain pipeline in this module — HmmBatchForwardF64
/// uses the main `hmm_forward_f64.wgsl` shader via `compile_shader_f64`.
pub const WGSL_HMM_FORWARD_LOG_F64: &str =
    include_str!("../../shaders/ml/hmm_forward_log_f64.wgsl");

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct HmmParams {
    n_states: u32,
    n_symbols: u32,
    n_steps: u32,
    n_seqs: u32,
}

/// Batch HMM forward pass on GPU.
pub struct HmmBatchForwardF64 {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
}

impl HmmBatchForwardF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let module = device.compile_shader_f64(SHADER, Some("hmm_forward_f64"));
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("HmmForward:bgl"),
                entries: &[
                    bgl_uniform(0),
                    bgl_storage(1, true),  // log_trans
                    bgl_storage(2, true),  // log_emit
                    bgl_storage(3, true),  // log_pi
                    bgl_storage(4, true),  // observations
                    bgl_storage(5, false), // log_alpha_out
                    bgl_storage(6, false), // log_lik_out
                ],
            });
        let layout = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("HmmForward:layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("HmmForward:pipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: "main",
                compilation_options: Default::default(),
                cache: None,
            });
        Ok(Self {
            device,
            pipeline,
            bgl,
        })
    }

    /// Dispatch the forward pass on GPU-resident buffers.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &self,
        n_states: u32,
        n_symbols: u32,
        n_steps: u32,
        n_seqs: u32,
        log_trans: &wgpu::Buffer,
        log_emit: &wgpu::Buffer,
        log_pi: &wgpu::Buffer,
        observations: &wgpu::Buffer,
        log_alpha_out: &wgpu::Buffer,
        log_lik_out: &wgpu::Buffer,
    ) -> Result<()> {
        let params = HmmParams {
            n_states,
            n_symbols,
            n_steps,
            n_seqs,
        };
        let params_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("HmmForward:params"),
            size: std::mem::size_of::<HmmParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.device
            .queue
            .write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

        let bg = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("HmmForward:bg"),
                layout: &self.bgl,
                entries: &[
                    bg_entry(0, &params_buf),
                    bg_entry(1, log_trans),
                    bg_entry(2, log_emit),
                    bg_entry(3, log_pi),
                    bg_entry(4, observations),
                    bg_entry(5, log_alpha_out),
                    bg_entry(6, log_lik_out),
                ],
            });

        let mut enc = self
            .device
            .device
            .create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(n_seqs.div_ceil(256), 1, 1);
        }
        self.device.submit_and_poll(Some(enc.finish()));
        Ok(())
    }
}

fn bg_entry(binding: u32, buf: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buf.as_entire_binding(),
    }
}

fn bgl_uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

fn bgl_storage(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_source_valid() {
        assert!(SHADER.contains("log_sum_exp2"));
        assert!(SHADER.contains("HmmParams"));
    }
}
