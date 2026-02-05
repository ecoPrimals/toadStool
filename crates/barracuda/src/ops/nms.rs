//! NMS - Non-Maximum Suppression (Pure GPU)
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! Filters overlapping bounding boxes in object detection.
//! Used in YOLO, Faster R-CNN, etc.
//!
//! Algorithm (Pure GPU):
//! 1. Compute IoU matrix on GPU (parallel)
//! 2. Sort indices by score (CPU - acceptable for small sets)
//! 3. Mark suppressed boxes on GPU (parallel)
//! 4. Compact results on GPU (parallel with atomics)

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Bounding box representation
#[derive(Clone, Debug)]
pub struct BoundingBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub score: f32,
}

/// NMS operation
pub struct NMS {
    boxes: Vec<BoundingBox>,
    iou_threshold: f32,
}

impl NMS {
    /// Create NMS operation
    pub fn new(boxes: Vec<BoundingBox>, iou_threshold: f32) -> Result<Self> {
        if iou_threshold < 0.0 || iou_threshold > 1.0 {
            return Err(BarracudaError::invalid_op(
                "NMS",
                format!("iou_threshold must be in [0, 1], got {}", iou_threshold),
            ));
        }

        Ok(Self {
            boxes,
            iou_threshold,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/nms.wgsl")
    }

    /// Execute NMS operation (Pure GPU)
    /// Returns indices of boxes to keep
    pub fn execute(self) -> Result<Vec<usize>> {
        if self.boxes.is_empty() {
            return Ok(Vec::new());
        }

        let num_boxes = self.boxes.len();

        // Edge case: single box
        if num_boxes == 1 {
            return Ok(vec![0]);
        }

        // Create device (blocking for sync context)
        let device = Arc::new(futures::executor::block_on(WgpuDevice::new())?);

        // Convert boxes to tensor format [num_boxes, 5] where each box is [x1, y1, x2, y2, score]
        let mut box_data = Vec::with_capacity(num_boxes * 5);
        for box_ in &self.boxes {
            box_data.push(box_.x1);
            box_data.push(box_.y1);
            box_data.push(box_.x2);
            box_data.push(box_.y2);
            box_data.push(box_.score);
        }

        // Create box tensor on GPU
        let boxes_tensor = futures::executor::block_on(Tensor::from_vec_on(
            box_data,
            vec![num_boxes, 5],
            device.clone(),
        ))?;

        // ====================================================================
        // Pass 1: Compute IoU Matrix on GPU
        // ====================================================================
        let iou_matrix_size = num_boxes * num_boxes;
        let iou_matrix_buffer = device.create_buffer_f32(iou_matrix_size)?;

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct IoUParams {
            num_boxes: u32,
            _padding: [u32; 3],
        }

        let iou_params = IoUParams {
            num_boxes: num_boxes as u32,
            _padding: [0; 3],
        };

        let iou_params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NMS IoU Params"),
            contents: bytemuck::cast_slice(&[iou_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Compile shader for IoU computation
        let shader_source = Self::wgsl_shader();
        let shader_module = device.compile_shader(shader_source, Some("NMS IoU Shader"));

        // Create bind group layout for IoU pass
        let iou_bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("NMS IoU Bind Group Layout"),
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

        let iou_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NMS IoU Bind Group"),
            layout: &iou_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: boxes_tensor.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: iou_matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: iou_params_buffer.as_entire_binding(),
                },
            ],
        });

        let iou_pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NMS IoU Pipeline Layout"),
            bind_group_layouts: &[&iou_bind_group_layout],
            push_constant_ranges: &[],
        });

        let iou_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NMS IoU Pipeline"),
            layout: Some(&iou_pipeline_layout),
            module: &shader_module,
            entry_point: "compute_iou_matrix",
        });

        // Execute IoU computation
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("NMS Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NMS IoU Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&iou_pipeline);
            compute_pass.set_bind_group(0, &iou_bind_group, &[]);
            // Dispatch with 16x16 workgroup size
            let workgroups_x = (num_boxes as u32 + 15) / 16;
            let workgroups_y = (num_boxes as u32 + 15) / 16;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // ====================================================================
        // Pass 2: Sort indices by score (CPU - acceptable for small sets)
        // ====================================================================
        let mut sorted_indices: Vec<u32> = (0..num_boxes as u32).collect();
        sorted_indices.sort_by(|&a, &b| {
            self.boxes[b as usize]
                .score
                .partial_cmp(&self.boxes[a as usize].score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Create sorted indices buffer
        let sorted_indices_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NMS Sorted Indices"),
            contents: bytemuck::cast_slice(&sorted_indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // ====================================================================
        // Pass 3: Mark Suppressed Boxes on GPU
        // ====================================================================
        let suppressed_buffer = device.create_buffer_u32_zeros(num_boxes)?;

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SuppressParams {
            num_boxes: u32,
            iou_threshold: f32,
            _padding: [u32; 2],
        }

        let suppress_params = SuppressParams {
            num_boxes: num_boxes as u32,
            iou_threshold: self.iou_threshold,
            _padding: [0; 2],
        };

        let suppress_params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NMS Suppress Params"),
            contents: bytemuck::cast_slice(&[suppress_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout for suppression pass
        let suppress_bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("NMS Suppress Bind Group Layout"),
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

        let suppress_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NMS Suppress Bind Group"),
            layout: &suppress_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sorted_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: iou_matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: suppressed_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: suppress_params_buffer.as_entire_binding(),
                },
            ],
        });

        let suppress_pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NMS Suppress Pipeline Layout"),
            bind_group_layouts: &[&suppress_bind_group_layout],
            push_constant_ranges: &[],
        });

        let suppress_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NMS Suppress Pipeline"),
            layout: Some(&suppress_pipeline_layout),
            module: &shader_module,
            entry_point: "mark_suppressed",
        });

        // Execute suppression marking
        let mut encoder2 = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("NMS Suppress Encoder"),
        });

        {
            let mut compute_pass = encoder2.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NMS Suppress Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&suppress_pipeline);
            compute_pass.set_bind_group(0, &suppress_bind_group, &[]);
            let workgroups = (num_boxes as u32 + 255) / 256;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder2.finish()));

        // ====================================================================
        // Pass 4: Compact Results on GPU
        // ====================================================================
        let keep_indices_buffer = device.create_buffer_u32(num_boxes)?;
        let keep_count_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NMS Keep Count"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CompactParams {
            num_boxes: u32,
            _padding: [u32; 3],
        }

        let compact_params = CompactParams {
            num_boxes: num_boxes as u32,
            _padding: [0; 3],
        };

        let compact_params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NMS Compact Params"),
            contents: bytemuck::cast_slice(&[compact_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout for compact pass
        let compact_bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("NMS Compact Bind Group Layout"),
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

        let compact_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NMS Compact Bind Group"),
            layout: &compact_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sorted_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: suppressed_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: keep_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: keep_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: compact_params_buffer.as_entire_binding(),
                },
            ],
        });

        let compact_pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NMS Compact Pipeline Layout"),
            bind_group_layouts: &[&compact_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compact_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NMS Compact Pipeline"),
            layout: Some(&compact_pipeline_layout),
            module: &shader_module,
            entry_point: "compact_results",
        });

        // Execute compaction
        let mut encoder3 = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("NMS Compact Encoder"),
        });

        {
            let mut compute_pass = encoder3.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NMS Compact Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compact_pipeline);
            compute_pass.set_bind_group(0, &compact_bind_group, &[]);
            let workgroups = (num_boxes as u32 + 255) / 256;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder3.finish()));

        // ====================================================================
        // Read Results (read_buffer_u32 will handle GPU synchronization)
        // ====================================================================
        let keep_count_data = device.read_buffer_u32(&keep_count_buffer, 1)?;
        let keep_count = keep_count_data[0] as usize;

        if keep_count == 0 {
            return Ok(Vec::new());
        }

        let keep_indices_data = device.read_buffer_u32(&keep_indices_buffer, keep_count)?;
        let keep: Vec<usize> = keep_indices_data.iter().map(|&idx| idx as usize).collect();

        Ok(keep)
    }

}

/// Compute IoU between two boxes (public for use by soft_nms)
pub fn compute_iou(box1: &BoundingBox, box2: &BoundingBox) -> f32 {
    let x1 = box1.x1.max(box2.x1);
    let y1 = box1.y1.max(box2.y1);
    let x2 = box1.x2.min(box2.x2);
    let y2 = box1.y2.min(box2.y2);

    let intersection = ((x2 - x1).max(0.0)) * ((y2 - y1).max(0.0));

    let area1 = (box1.x2 - box1.x1) * (box1.y2 - box1.y1);
    let area2 = (box2.x2 - box2.x1) * (box2.y2 - box2.y1);
    let union = area1 + area2 - intersection;

    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

/// Convenience function for NMS
pub fn nms(
    boxes: Vec<BoundingBox>,
    iou_threshold: f32,
) -> Result<Vec<usize>> {
    NMS::new(boxes, iou_threshold)?.execute()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nms_basic() {
        let boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 1.0,
                y1: 1.0,
                x2: 11.0,
                y2: 11.0,
                score: 0.8,
            }, // Overlaps
        ];
        let keep = nms(boxes, 0.5).unwrap();
        assert_eq!(keep.len(), 1); // Second box suppressed
        assert_eq!(keep[0], 0); // Highest score kept
    }

    #[test]
    fn test_nms_edge_cases() {
        // No overlapping boxes (all kept)
        let boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 20.0,
                y1: 20.0,
                x2: 30.0,
                y2: 30.0,
                score: 0.8,
            },
            BoundingBox {
                x1: 40.0,
                y1: 40.0,
                x2: 50.0,
                y2: 50.0,
                score: 0.7,
            },
        ];
        let keep = nms(boxes, 0.5).unwrap();
        assert_eq!(keep.len(), 3); // All boxes kept

        // Single box
        let boxes = vec![BoundingBox {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
            score: 0.9,
        }];
        let keep = nms(boxes, 0.5).unwrap();
        assert_eq!(keep.len(), 1);
    }

    #[test]
    fn test_nms_boundary() {
        // Test with overlapping boxes
        let boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 5.0,
                y1: 0.0,
                x2: 15.0,
                y2: 10.0,
                score: 0.8,
            },
        ];

        // Very strict threshold (keep everything)
        let keep = nms(boxes.clone(), 0.99).unwrap();
        assert_eq!(keep.len(), 2); // Both kept

        // Very loose threshold (suppress aggressively)
        let keep = nms(boxes, 0.01).unwrap();
        assert_eq!(keep.len(), 1); // Only highest score
    }

    #[test]
    fn test_nms_empty() {
        let keep = nms(vec![], 0.5).unwrap();
        assert_eq!(keep.len(), 0);
    }
}
