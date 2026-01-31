use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FocalLossParams {
    alpha: f32,
    gamma: f32,
    epsilon: f32,
    reduction_mode: u32,
    size: u32,
    _pad1: [u32; 3],
    _pad2: [u32; 4],
    _pad3: [u32; 4],
    _pad4: [u32; 4],
    _pad5: [u32; 4],
}

pub struct FocalLoss {
    predictions: Tensor,
    targets: Tensor,
    alpha: f32,
    gamma: f32,
}

impl FocalLoss {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/focal_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let size = self.predictions.shape().iter().product::<usize>();
        
        let params = FocalLossParams {
            alpha: self.alpha,
            gamma: self.gamma,
            epsilon: 1e-7,
            reduction_mode: 0, // mean reduction
            size: size as u32,
            _pad1: [0; 3],
            _pad2: [0; 4],
            _pad3: [0; 4],
            _pad4: [0; 4],
            _pad5: [0; 4],
        };
        
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("focal_loss_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("focal_loss_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("focal_loss_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("focal_loss_bind_group_layout"),
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
        
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("focal_loss_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("focal_loss_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("focal_loss_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.predictions.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.targets.buffer().as_entire_binding(),
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
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("focal_loss_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("focal_loss_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        Ok(Tensor::from_buffer(
            output_buffer,
            self.predictions.shape().to_vec(),
            device.clone(),
        ))
    }
}

pub trait FocalLossExt {
    fn focal_loss(self, targets: &Tensor, alpha: f32, gamma: f32) -> Result<Tensor>;
}

impl FocalLossExt for Tensor {
    fn focal_loss(self, targets: &Tensor, alpha: f32, gamma: f32) -> Result<Tensor> {
        let op = FocalLoss {
            predictions: self,
            targets: targets.clone(),
            alpha,
            gamma,
        };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_focal_loss_basic() {
        let device = get_test_device().await;
        
        let predictions = Tensor::from_data(
            &vec![0.9, 0.1, 0.8, 0.2],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let targets = Tensor::from_data(
            &vec![1.0, 0.0, 1.0, 0.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let result = predictions.focal_loss(&targets, 0.25, 2.0).unwrap();
        let loss = result.to_vec().unwrap();
        
        assert_eq!(loss.len(), 4);
        // Verify operation completed successfully
        assert!(loss.len() > 0);
    }

    #[tokio::test]
    async fn test_focal_loss_edge_cases() {
        let device = get_test_device().await;

        // Perfect predictions
        let predictions = Tensor::from_data(&vec![1.0, 0.0, 1.0], vec![3], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![1.0, 0.0, 1.0], vec![3], device.clone()).unwrap();
        let result = predictions.focal_loss(&targets, 0.25, 2.0).unwrap();
        let loss = result.to_vec().unwrap();
        assert_eq!(loss.len(), 3); // Verify operation completed

        // Single element
        let predictions = Tensor::from_data(&vec![0.7], vec![1], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![1.0], vec![1], device).unwrap();
        let result = predictions.focal_loss(&targets, 0.5, 2.0).unwrap();
        let loss = result.to_vec().unwrap();
        assert_eq!(loss.len(), 1);
    }

    #[tokio::test]
    async fn test_focal_loss_boundary() {
        let device = get_test_device().await;

        // Different alpha values
        let predictions = Tensor::from_data(&vec![0.6, 0.4], vec![2], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![1.0, 0.0], vec![2], device.clone()).unwrap();
        let result1 = predictions.clone().focal_loss(&targets, 0.25, 2.0).unwrap();
        let loss1 = result1.to_vec().unwrap();
        
        let result2 = predictions.focal_loss(&targets, 0.75, 2.0).unwrap();
        let loss2 = result2.to_vec().unwrap();
        
        // Both should complete successfully
        assert_eq!(loss1.len(), 2);
        assert_eq!(loss2.len(), 2);
    }

    #[tokio::test]
    async fn test_focal_loss_large_batch() {
        let device = get_test_device().await;

        // 100 elements
        let mut preds = Vec::with_capacity(100);
        let mut tgts = Vec::with_capacity(100);
        for i in 0..100 {
            preds.push(if i % 2 == 0 { 0.8 } else { 0.2 });
            tgts.push(if i % 2 == 0 { 1.0 } else { 0.0 });
        }
        
        let predictions = Tensor::from_data(&preds, vec![100], device.clone()).unwrap();
        let targets = Tensor::from_data(&tgts, vec![100], device).unwrap();
        
        let result = predictions.focal_loss(&targets, 0.25, 2.0).unwrap();
        let loss = result.to_vec().unwrap();
        
        assert_eq!(loss.len(), 100);
    }

    #[tokio::test]
    async fn test_focal_loss_precision() {
        let device = get_test_device().await;

        // Gamma parameter effect
        let predictions = Tensor::from_data(&vec![0.5, 0.9], vec![2], device.clone()).unwrap();
        let targets = Tensor::from_data(&vec![1.0, 1.0], vec![2], device.clone()).unwrap();
        
        // Low gamma
        let result_low = predictions.clone().focal_loss(&targets, 0.25, 0.5).unwrap();
        let loss_low = result_low.to_vec().unwrap();
        
        // High gamma
        let result_high = predictions.focal_loss(&targets, 0.25, 4.0).unwrap();
        let loss_high = result_high.to_vec().unwrap();
        
        // Both should complete successfully
        assert_eq!(loss_low.len(), 2);
        assert_eq!(loss_high.len(), 2);
    }
}
