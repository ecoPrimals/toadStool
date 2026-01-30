use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AdaDeltaParams {
    rho: f32,
    epsilon: f32,
    weight_decay: f32,
    _padding: u32,
}

pub struct AdaDelta {
    weights: Tensor,
    gradients: Tensor,
    acc_grad: Option<Tensor>,
    acc_delta: Option<Tensor>,
    rho: f32,
}

impl AdaDelta {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/adadelta.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor, Tensor)> {
        let device = self.weights.device();
        let size = self.weights.shape().iter().product::<usize>();
        
        let params = AdaDeltaParams {
            rho: self.rho,
            epsilon: 1e-6,
            weight_decay: 0.0,
            _padding: 0,
        };
        
        // Create state buffers if not provided
        let zeros = vec![0.0f32; size];
        let acc_grad_in = if let Some(ref tensor) = self.acc_grad {
            tensor.buffer()
        } else {
            &device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adadelta_acc_grad_zeros"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        
        let acc_delta_in = if let Some(ref tensor) = self.acc_delta {
            tensor.buffer()
        } else {
            &device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adadelta_acc_delta_zeros"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        
        let weights_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adadelta_weights_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let acc_grad_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adadelta_acc_grad_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let acc_delta_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adadelta_acc_delta_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("adadelta_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("adadelta_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("adadelta_bind_group_layout"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
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
            label: Some("adadelta_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("adadelta_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adadelta_bind_group"),
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
                    resource: acc_grad_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: acc_delta_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: acc_grad_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: acc_delta_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("adadelta_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adadelta_pass"),
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
        
        let updated_acc_grad = Tensor::from_buffer(
            acc_grad_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );
        
        let updated_acc_delta = Tensor::from_buffer(
            acc_delta_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );
        
        Ok((updated_weights, updated_acc_grad, updated_acc_delta))
    }
}

pub trait AdaDeltaExt {
    fn adadelta_step(self, gradients: &Tensor, rho: f32, acc_grad: Option<&Tensor>, acc_delta: Option<&Tensor>) -> Result<(Tensor, Tensor, Tensor)>;
}

impl AdaDeltaExt for Tensor {
    fn adadelta_step(self, gradients: &Tensor, rho: f32, acc_grad: Option<&Tensor>, acc_delta: Option<&Tensor>) -> Result<(Tensor, Tensor, Tensor)> {
        let op = AdaDelta {
            weights: self,
            gradients: gradients.clone(),
            acc_grad: acc_grad.cloned(),
            acc_delta: acc_delta.cloned(),
            rho,
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
    async fn test_adadelta_basic() {
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
        
        let (updated_weights, _acc_grad, _acc_delta) = weights.adadelta_step(&gradients, 0.95, None, None).unwrap();
        let result = updated_weights.to_vec().unwrap();
        
        // Weights should be updated
        assert_eq!(result.len(), 4);
        assert!(result[0] < 1.0);
    }
}
