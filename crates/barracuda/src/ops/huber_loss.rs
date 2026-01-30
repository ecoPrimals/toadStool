use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct HuberLossParams {
    delta: f32,
    reduction_mode: u32,
    size: u32,
    _padding: u32,
}

pub struct HuberLoss {
    predictions: Tensor,
    targets: Tensor,
    delta: f32,
}

impl HuberLoss {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/huber_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let size = self.predictions.shape().iter().product::<usize>();
        
        let params = HuberLossParams {
            delta: self.delta,
            reduction_mode: 0, // mean reduction
            size: size as u32,
            _padding: 0,
        };
        
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("huber_loss_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("huber_loss_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("huber_loss_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("huber_loss_bind_group_layout"),
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
            label: Some("huber_loss_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("huber_loss_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("huber_loss_bind_group"),
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
            label: Some("huber_loss_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("huber_loss_pass"),
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

pub trait HuberLossExt {
    fn huber_loss(self, targets: &Tensor, delta: f32) -> Result<Tensor>;
}

impl HuberLossExt for Tensor {
    fn huber_loss(self, targets: &Tensor, delta: f32) -> Result<Tensor> {
        let op = HuberLoss {
            predictions: self,
            targets: targets.clone(),
            delta,
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
    async fn test_huber_loss_small_errors() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        // Small errors (< delta): should use quadratic (MSE-like)
        let predictions = Tensor::from_data(
            &vec![1.0, 2.0, 3.0, 4.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let targets = Tensor::from_data(
            &vec![1.1, 2.1, 2.9, 3.9],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let result = predictions.huber_loss(&targets, 1.0).unwrap();
        let loss = result.to_vec().unwrap();
        
        assert_eq!(loss.len(), 4);
        // All errors = 0.1, which is < delta=1.0
        // Loss should be 0.5 * 0.1^2 = 0.005
        for &l in &loss {
            assert!((l - 0.005).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_huber_loss_large_errors() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        // Large errors (> delta): should use linear (MAE-like)
        let predictions = Tensor::from_data(
            &vec![1.0, 2.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        let targets = Tensor::from_data(
            &vec![3.0, 5.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        let result = predictions.huber_loss(&targets, 1.0).unwrap();
        let loss = result.to_vec().unwrap();
        
        assert_eq!(loss.len(), 2);
        // Error 1: |1-3| = 2 > delta=1, loss = 1*(2 - 0.5*1) = 1.5
        assert!((loss[0] - 1.5).abs() < 1e-5);
        // Error 2: |2-5| = 3 > delta=1, loss = 1*(3 - 0.5*1) = 2.5
        assert!((loss[1] - 2.5).abs() < 1e-5);
    }
}
