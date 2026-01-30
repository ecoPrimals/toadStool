use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AdaGradParams {
    learning_rate: f32,
    epsilon: f32,
    weight_decay: f32,
    _padding: u32,
}

pub struct AdaGrad {
    weights: Tensor,
    gradients: Tensor,
    accumulated: Option<Tensor>,
    learning_rate: f32,
}

impl AdaGrad {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/adagrad.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor)> {
        let device = self.weights.device();
        let size = self.weights.shape().iter().product::<usize>();
        
        let params = AdaGradParams {
            learning_rate: self.learning_rate,
            epsilon: 1e-8,
            weight_decay: 0.0,
            _padding: 0,
        };
        
        // Create accumulated buffer if not provided
        let zeros = vec![0.0f32; size];
        let accumulated_in = if let Some(ref acc_tensor) = self.accumulated {
            acc_tensor.buffer()
        } else {
            &device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adagrad_acc_zeros"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        
        let weights_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adagrad_weights_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let accumulated_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adagrad_accumulated_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("adagrad_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("adagrad_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("adagrad_bind_group_layout"),
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
            label: Some("adagrad_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("adagrad_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adagrad_bind_group"),
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
                    resource: accumulated_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: accumulated_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("adagrad_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adagrad_pass"),
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
        
        let updated_accumulated = Tensor::from_buffer(
            accumulated_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );
        
        Ok((updated_weights, updated_accumulated))
    }
}

pub trait AdaGradExt {
    fn adagrad_step(self, gradients: &Tensor, learning_rate: f32, accumulated: Option<&Tensor>) -> Result<(Tensor, Tensor)>;
}

impl AdaGradExt for Tensor {
    fn adagrad_step(self, gradients: &Tensor, learning_rate: f32, accumulated: Option<&Tensor>) -> Result<(Tensor, Tensor)> {
        let op = AdaGrad {
            weights: self,
            gradients: gradients.clone(),
            accumulated: accumulated.cloned(),
            learning_rate,
        };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_adagrad_basic() {
        let device = get_test_device().await;
        
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
        
        let (updated_weights, _acc) = weights.adagrad_step(&gradients, 0.01, None).unwrap();
        let result = updated_weights.to_vec().unwrap();
        
        // Weights should be updated
        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(result[0] < 1.0);
        assert!(result[1] < 2.0);
    }

    #[tokio::test]
    async fn test_adagrad_edge_cases() {
        let device = get_test_device().await;
        
        // Test with zero gradients
        let weights = Tensor::from_data(
            &vec![1.0, 2.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &vec![0.0, 0.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        let (updated_weights, acc) = weights.adagrad_step(&gradients, 0.01, None).unwrap();
        let result = updated_weights.to_vec().unwrap();
        let acc_result = acc.to_vec().unwrap();
        
        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(acc_result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adagrad_boundary() {
        let device = get_test_device().await;
        
        // Test with different learning rates
        let weights1 = Tensor::from_data(
            &vec![1.0; 4],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let weights2 = Tensor::from_data(
            &vec![1.0; 4],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &vec![0.1; 4],
            vec![4],
            device.clone(),
        ).unwrap();
        
        // Small learning rate
        let (updated1, _acc) = weights1.adagrad_step(&gradients, 0.001, None).unwrap();
        
        // Large learning rate
        let (updated2, _acc) = weights2.adagrad_step(&gradients, 0.1, None).unwrap();
        
        let result1 = updated1.to_vec().unwrap();
        let result2 = updated2.to_vec().unwrap();
        
        assert!(result1.iter().all(|&x| x.is_finite()));
        assert!(result2.iter().all(|&x| x.is_finite()));
        // Both should be valid updates
        assert!(result1[0] < 1.0);
        assert!(result2[0] < 1.0);
    }

    #[tokio::test]
    async fn test_adagrad_large_batch() {
        let device = get_test_device().await;
        
        // Larger parameter set
        let size = 128;
        let weights_data: Vec<f32> = (0..size).map(|i| (i as f32) / 10.0).collect();
        let grads_data = vec![0.01; size];
        
        let weights = Tensor::from_data(
            &weights_data,
            vec![size],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &grads_data,
            vec![size],
            device.clone(),
        ).unwrap();
        
        let (updated_weights, updated_acc) = weights.adagrad_step(&gradients, 0.01, None).unwrap();
        
        let result = updated_weights.to_vec().unwrap();
        let acc = updated_acc.to_vec().unwrap();
        
        assert_eq!(result.len(), size);
        assert_eq!(acc.len(), size);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adagrad_precision() {
        let device = get_test_device().await;
        
        // Test accumulated gradient behavior
        let weights = Tensor::from_data(
            &vec![10.0, 20.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &vec![1.0, 2.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        // Step 1
        let (weights1, acc1) = weights.adagrad_step(&gradients, 0.1, None).unwrap();
        let result1 = weights1.to_vec().unwrap();
        
        assert!(result1[0] < 10.0);
        assert!(result1[1] < 20.0);
        
        // Step 2 with accumulated gradients
        let (weights2, _acc2) = weights1.adagrad_step(&gradients, 0.1, Some(&acc1)).unwrap();
        let result2 = weights2.to_vec().unwrap();
        
        // Should continue optimizing
        assert!(result2.iter().all(|&x| x.is_finite()));
        // Should still be decreasing from original
        assert!(result2[0] < 10.0);
        assert!(result2[1] < 20.0);
    }
}
