//! Generalized IoU Loss for object detection
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Improves upon IoU by considering the smallest enclosing box

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GIoULossParams {
    num_boxes: u32,
    box_format: u32, // 0 = xyxy, 1 = xywh, 2 = cxcywh
    _padding: [u32; 2],
}

pub struct GIoULoss {
    pred_boxes: Tensor,
    target_boxes: Tensor,
    box_format: u32,
}

impl GIoULoss {
    /// Create GIoULoss operation
    pub fn new(pred_boxes: Tensor, target_boxes: Tensor, box_format: u32) -> Result<Self> {
        if box_format > 2 {
            return Err(BarracudaError::invalid_op(
                "GIoULoss",
                format!(
                    "box_format must be 0 (xyxy), 1 (xywh), or 2 (cxcywh), got {}",
                    box_format
                ),
            ));
        }

        Ok(Self {
            pred_boxes,
            target_boxes,
            box_format,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/loss/giou_loss.wgsl")
    }

    /// Execute GIoULoss on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.pred_boxes.device();
        let pred_shape = self.pred_boxes.shape();
        let target_shape = self.target_boxes.shape();

        if pred_shape.len() != 2 || target_shape.len() != 2 {
            return Err(BarracudaError::invalid_op(
                "GIoULoss",
                format!(
                    "boxes must be 2D [num_boxes, 4], got shapes {:?} and {:?}",
                    pred_shape, target_shape
                ),
            ));
        }

        if pred_shape[1] != 4 || target_shape[1] != 4 {
            return Err(BarracudaError::invalid_op(
                "GIoULoss",
                format!(
                    "boxes must have 4 coordinates, got {} and {}",
                    pred_shape[1], target_shape[1]
                ),
            ));
        }

        if pred_shape[0] != target_shape[0] {
            return Err(BarracudaError::invalid_op(
                "GIoULoss",
                format!(
                    "pred and target must have same number of boxes: {} != {}",
                    pred_shape[0], target_shape[0]
                ),
            ));
        }

        let num_boxes = pred_shape[0];

        // Create output buffer: [num_boxes]
        let output_buffer = device.create_buffer_f32(num_boxes)?;

        let params = GIoULossParams {
            num_boxes: num_boxes as u32,
            box_format: self.box_format,
            _padding: [0; 2],
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GIoULoss Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("GIoULoss Bind Group Layout"),
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
            label: Some("GIoULoss Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.pred_boxes.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.target_boxes.buffer().as_entire_binding(),
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
        let shader = device.compile_shader(Self::wgsl_shader(), Some("GIoULoss"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("GIoULoss Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("GIoULoss Pipeline"),
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
                label: Some("GIoULoss Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GIoULoss Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (num_boxes as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![num_boxes],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_giou_loss_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let num_boxes = 3;

        let pred_boxes = Tensor::from_vec_on(
            vec![
                0.0, 0.0, 10.0, 10.0, 5.0, 5.0, 15.0, 15.0, 10.0, 10.0, 20.0, 20.0,
            ],
            vec![num_boxes, 4],
            device.clone(),
        )
        .await
        .unwrap();

        let target_boxes = Tensor::from_vec_on(
            vec![
                1.0, 1.0, 11.0, 11.0, 6.0, 6.0, 16.0, 16.0, 11.0, 11.0, 21.0, 21.0,
            ],
            vec![num_boxes, 4],
            device.clone(),
        )
        .await
        .unwrap();

        let result = GIoULoss::new(pred_boxes, target_boxes, 0)
            .unwrap()
            .execute()
            .unwrap();

        assert_eq!(result.shape(), &[num_boxes]);
    }
}
