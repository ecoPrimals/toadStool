use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SGDParams {
    learning_rate: f32,
    momentum: f32,
    weight_decay: f32,
    dampening: f32,
}

pub struct SGD {
    weights: Tensor,
    gradients: Tensor,
    velocity: Option<Tensor>,
    learning_rate: f32,
    momentum: f32,
    weight_decay: f32,
}

impl SGD {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/sgd.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Option<Tensor>)> {
        let device = self.weights.device();
        let size = self.weights.shape().iter().product::<usize>();
        
        let params = SGDParams {
            learning_rate: self.learning_rate,
            momentum: self.momentum,
            weight_decay: self.weight_decay,
            dampening: 0.0,
        };
        
        // Create velocity buffer if not provided
        let velocity_in = if let Some(ref v) = self.velocity {
            v.buffer()
        } else {
            let zeros = &vec![0.0f32; size];
            &device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sgd_velocity_zeros"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        
        let weights_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sgd_weights_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let velocity_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sgd_velocity_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = &device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sgd_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sgd_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sgd_bind_group_layout"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
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
            label: Some("sgd_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sgd_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sgd_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.weights.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.gradients.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: velocity_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: velocity_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sgd_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sgd_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        let updated_weights = Tensor::from_buffer(
            weights_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );
        
        let updated_velocity = if self.momentum != 0.0 {
            Some(Tensor::from_buffer(
                velocity_out_buffer,
                self.weights.shape().to_vec(),
                device.clone(),
            ))
        } else {
            None
        };
        
        Ok((updated_weights, updated_velocity))
    }
}

pub trait SGDExt {
    fn sgd_step(self, gradients: &Tensor, learning_rate: f32, momentum: f32, weight_decay: f32, velocity: Option<&Tensor>) -> Result<(Tensor, Option<Tensor>)>;
}

impl SGDExt for Tensor {
    fn sgd_step(self, gradients: &Tensor, learning_rate: f32, momentum: f32, weight_decay: f32, velocity: Option<&Tensor>) -> Result<(Tensor, Option<Tensor>)> {
        let op = SGD {
            weights: self,
            gradients: gradients.clone(),
            velocity: velocity.cloned(),
            learning_rate,
            momentum,
            weight_decay,
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
    async fn test_sgd_basic() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        let weights = Tensor::from_data(
            &vec![1.0, 2.0, 3.0, 4.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &vec![0.1, 0.2, 0.3, 0.4],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let (updated_weights, _) = weights.sgd_step(&gradients, 0.1, 0.0, 0.0, None).unwrap();
        let result = updated_weights.to_vec().unwrap();
        
        // weights - lr * gradients
        assert!((result[0] - (1.0 - 0.1 * 0.1)).abs() < 1e-5); // 1.0 - 0.01 = 0.99
        assert!((result[1] - (2.0 - 0.1 * 0.2)).abs() < 1e-5); // 2.0 - 0.02 = 1.98
        assert!((result[2] - (3.0 - 0.1 * 0.3)).abs() < 1e-5); // 3.0 - 0.03 = 2.97
        assert!((result[3] - (4.0 - 0.1 * 0.4)).abs() < 1e-5); // 4.0 - 0.04 = 3.96
    }
}
