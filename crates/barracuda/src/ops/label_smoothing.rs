//! Label Smoothing for classification
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Prevents overconfidence by smoothing hard labels

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LabelSmoothingParams {
    batch_size: u32,
    num_classes: u32,
    smoothing: f32,
    _padding: u32,
}

pub struct LabelSmoothing {
    labels: Tensor,
    num_classes: u32,
    smoothing: f32,
}

impl LabelSmoothing {
    /// Create LabelSmoothing operation
    pub fn new(labels: Tensor, num_classes: u32, smoothing: f32) -> Result<Self> {
        if !(0.0..=1.0).contains(&smoothing) {
            return Err(BarracudaError::invalid_op(
                "LabelSmoothing",
                format!("smoothing must be in [0, 1], got {}", smoothing),
            ));
        }

        Ok(Self {
            labels,
            num_classes,
            smoothing,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        static SHADER: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| crate::shaders::precision::downcast_f64_to_f32_with_transcendentals(include_str!("../shaders/loss/label_smoothing_f64.wgsl")));
        &SHADER
    }

    /// Execute LabelSmoothing on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.labels.device();
        let labels_shape = self.labels.shape();

        if labels_shape.len() != 1 {
            return Err(BarracudaError::invalid_op(
                "LabelSmoothing",
                format!(
                    "labels must be 1D [batch_size], got shape {:?}",
                    labels_shape
                ),
            ));
        }

        let batch_size = labels_shape[0];

        // Create output buffer: [batch_size, num_classes]
        let output_size = batch_size * self.num_classes as usize;
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = LabelSmoothingParams {
            batch_size: batch_size as u32,
            num_classes: self.num_classes,
            smoothing: self.smoothing,
            _padding: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LabelSmoothing Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LabelSmoothing Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LabelSmoothing Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.labels.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("LabelSmoothing"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("LabelSmoothing Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("LabelSmoothing Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LabelSmoothing Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LabelSmoothing Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let workgroups = (batch_size as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, self.num_classes as usize],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_label_smoothing_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let batch_size = 4;
        let num_classes = 3;

        let labels =
            Tensor::from_vec_on(vec![0.0, 1.0, 2.0, 0.0], vec![batch_size], device.clone())
                .await
                .unwrap();

        let result = LabelSmoothing::new(labels, num_classes, 0.1)
            .unwrap()
            .execute()
            .unwrap();

        assert_eq!(result.shape(), &[batch_size, num_classes as usize]);
    }
}
