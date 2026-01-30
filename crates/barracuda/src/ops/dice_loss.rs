use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DiceLossParams {
    smoothing: f32,
    reduction_mode: u32,
    batch_size: u32,
    elements_per_sample: u32,
}

pub struct DiceLoss {
    predictions: Tensor,
    targets: Tensor,
    smoothing: f32,
}

impl DiceLoss {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/dice_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let shape = self.predictions.shape();
        
        if shape.len() < 2 {
            return Err(crate::error::BarracudaError::invalid_op("Shape Error", 
                format!("DiceLoss expects at least 2D input [batch, ...], got shape {:?}", shape)
            ));
        }
        
        let batch_size = shape[0];
        let elements_per_sample = shape[1..].iter().product::<usize>();
        
        let params = DiceLossParams {
            smoothing: self.smoothing,
            reduction_mode: 0, // mean reduction
            batch_size: batch_size as u32,
            elements_per_sample: elements_per_sample as u32,
        };
        
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dice_loss_output"),
            size: (batch_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dice_loss_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dice_loss_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dice_loss_bind_group_layout"),
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
            label: Some("dice_loss_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("dice_loss_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dice_loss_bind_group"),
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
            label: Some("dice_loss_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("dice_loss_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            compute_pass.dispatch_workgroups(batch_size as u32, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size],
            device.clone(),
        ))
    }
}

pub trait DiceLossExt {
    fn dice_loss(self, targets: &Tensor, smoothing: f32) -> Result<Tensor>;
}

impl DiceLossExt for Tensor {
    fn dice_loss(self, targets: &Tensor, smoothing: f32) -> Result<Tensor> {
        let op = DiceLoss {
            predictions: self,
            targets: targets.clone(),
            smoothing,
        };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_dice_loss_perfect_overlap() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        // Perfect overlap: loss should be ~0
        let predictions = Tensor::from_data(
            &vec![1.0, 1.0, 0.0, 0.0],
            vec![1, 4],
            device.clone(),
        ).unwrap();
        
        let targets = Tensor::from_data(
            &vec![1.0, 1.0, 0.0, 0.0],
            vec![1, 4],
            device.clone(),
        ).unwrap();
        
        let result = predictions.dice_loss(&targets, 1.0).unwrap();
        let loss = result.to_vec().unwrap();
        
        assert_eq!(loss.len(), 1);
        assert!(loss[0] < 0.1); // Should be close to 0
    }

    #[tokio::test]
    async fn test_dice_loss_no_overlap() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        // No overlap: loss should be ~1
        let predictions = Tensor::from_data(
            &vec![1.0, 1.0, 0.0, 0.0],
            vec![1, 4],
            device.clone(),
        ).unwrap();
        
        let targets = Tensor::from_data(
            &vec![0.0, 0.0, 1.0, 1.0],
            vec![1, 4],
            device.clone(),
        ).unwrap();
        
        let result = predictions.dice_loss(&targets, 1.0).unwrap();
        let loss = result.to_vec().unwrap();
        
        assert_eq!(loss.len(), 1);
        assert!(loss[0] > 0.9); // Should be close to 1
    }
}
