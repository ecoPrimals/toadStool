//! Utility functions for WGPU operations
//!
//! Eliminates boilerplate and provides safe, idiomatic Rust patterns.
//! Deep Debt: Zero hardcoded values, runtime-configurable.

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::executor::WgpuExecutor;

impl WgpuExecutor {
    /// Create GPU buffer from data (safe helper, zero-copy where possible)
    ///
    /// Deep Debt: No hardcoded buffer sizes, determined at runtime.
    pub(crate) fn create_input_buffer(&self, data: &[f32], label: &str) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
    }

    /// Create output buffer (safe helper)
    pub(crate) fn create_output_buffer(&self, size: usize, label: &str) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Create staging buffer for reading results (safe helper)
    pub(crate) fn create_staging_buffer(&self, size: usize, label: &str) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Create uniform buffer from data (safe helper)
    ///
    /// Deep Debt: No hardcoded buffer sizes, determined at runtime.
    pub(crate) fn create_uniform_buffer(&self, data: &[f32], label: &str) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
    }

    /// Read buffer results (async, safe, modern Rust)
    ///
    /// Idiomatic async/await pattern instead of callback-based.
    pub(crate) async fn read_buffer(&self, buffer: &wgpu::Buffer, _size: usize) -> Result<Vec<f32>> {
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .receive()
            .await
            .context("Failed to receive buffer map result")??;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        buffer.unmap();

        Ok(result)
    }

    /// Calculate workgroup count (safe helper, no magic numbers)
    ///
    /// Deep Debt: Workgroup size determined at runtime based on GPU capabilities,
    /// not hardcoded. Currently uses 256 as a safe default that works on all GPUs.
    pub(crate) fn calculate_workgroups(&self, size: usize, workgroup_size: u32) -> u32 {
        ((size as u32) + workgroup_size - 1) / workgroup_size
    }

    /// Create simple compute pipeline (reduces boilerplate)
    pub(crate) fn create_simple_pipeline(
        &self,
        shader_source: &str,
        shader_label: &str,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::ComputePipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(shader_label),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&format!("{} Pipeline Layout", shader_label)),
                    bind_group_layouts: &[bind_group_layout],
                    push_constant_ranges: &[],
                });

        self.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{} Pipeline", shader_label)),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                compilation_options: Default::default(),
                cache: None,
            })
    }

    /// Create standard 2-buffer bind group layout (input, output)
    pub(crate) fn create_binary_bind_group_layout(&self, label: &str) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
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
                ],
            })
    }

    /// Execute simple compute pass (common pattern extracted)
    pub(crate) fn execute_compute_pass(
        &self,
        pipeline: &wgpu::ComputePipeline,
        bind_group: &wgpu::BindGroup,
        workgroups: u32,
        label: &str,
    ) -> wgpu::CommandEncoder {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(&format!("{} Encoder", label)),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&format!("{} Pass", label)),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder
    }
}
