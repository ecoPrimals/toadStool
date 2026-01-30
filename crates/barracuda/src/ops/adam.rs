use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AdamParams {
    num_params: u32,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
    step: u32,
}

pub struct Adam {
    gradients: Tensor,
    params: Tensor,
    m: Option<Tensor>,
    v: Option<Tensor>,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    step: usize,
}

impl Adam {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/adam.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor, Tensor)> {
        let device = self.params.device();
        let size = self.params.shape().iter().product::<usize>();
        
        let adam_params = AdamParams {
            num_params: size as u32,
            learning_rate: self.learning_rate,
            beta1: self.beta1,
            beta2: self.beta2,
            epsilon: 1e-8,
            weight_decay: 0.0,
            step: self.step as u32,
        };
        
        // Create m and v buffers if not provided
        let zeros = vec![0.0f32; size];
        let m_buffer = if let Some(ref m_tensor) = self.m {
            m_tensor.buffer()
        } else {
            &device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adam_m_zeros"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
        };
        
        let v_buffer = if let Some(ref v_tensor) = self.v {
            v_tensor.buffer()
        } else {
            &device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adam_v_zeros"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
        };
        
        // Create output buffers (params will be updated in-place, but we need new m/v)
        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adam_params_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let m_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adam_m_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let v_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adam_v_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Copy input params to output buffer (since shader updates in-place)
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("adam_copy_encoder"),
        });
        encoder.copy_buffer_to_buffer(
            self.params.buffer(),
            0,
            &params_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            m_buffer,
            0,
            &m_out_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            v_buffer,
            0,
            &v_out_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        device.queue.submit(Some(encoder.finish()));
        
        let params_uniform = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("adam_params"),
            contents: bytemuck::cast_slice(&[adam_params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("adam_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("adam_bind_group_layout"),
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
        
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("adam_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("adam_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adam_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.gradients.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: m_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: v_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_uniform.as_entire_binding(),
                },
            ],
        });
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("adam_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adam_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        let updated_params = Tensor::from_buffer(
            params_buffer,
            self.params.shape().to_vec(),
            device.clone(),
        );
        
        let updated_m = Tensor::from_buffer(
            m_out_buffer,
            self.params.shape().to_vec(),
            device.clone(),
        );
        
        let updated_v = Tensor::from_buffer(
            v_out_buffer,
            self.params.shape().to_vec(),
            device.clone(),
        );
        
        Ok((updated_params, updated_m, updated_v))
    }
}

pub trait AdamExt {
    fn adam_step(self, gradients: &Tensor, learning_rate: f32, beta1: f32, beta2: f32, step: usize, m: Option<&Tensor>, v: Option<&Tensor>) -> Result<(Tensor, Tensor, Tensor)>;
}

impl AdamExt for Tensor {
    fn adam_step(self, gradients: &Tensor, learning_rate: f32, beta1: f32, beta2: f32, step: usize, m: Option<&Tensor>, v: Option<&Tensor>) -> Result<(Tensor, Tensor, Tensor)> {
        let op = Adam {
            gradients: gradients.clone(),
            params: self,
            m: m.cloned(),
            v: v.cloned(),
            learning_rate,
            beta1,
            beta2,
            step,
        };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_adam_basic() {
        let device = get_test_device().await;
        
        let params = Tensor::from_data(
            &vec![1.0, 2.0, 3.0, 4.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &vec![0.1, 0.2, 0.3, 0.4],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let (updated_params, _m, _v) = params.adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None).unwrap();
        let result = updated_params.to_vec().unwrap();
        
        // Params should be updated with Adam optimizer
        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));
        // Check that params decreased (gradients are positive, small LR)
        assert!(result[0] < 1.0);
        assert!(result[1] < 2.0);
    }

    #[tokio::test]
    async fn test_adam_edge_cases() {
        let device = get_test_device().await;
        
        // Test with zero gradients
        let params = Tensor::from_data(
            &vec![1.0, 2.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &vec![0.0, 0.0],
            vec![2],
            device.clone(),
        ).unwrap();
        
        let (updated_params, m, v) = params.adam_step(&gradients, 0.01, 0.9, 0.999, 1, None, None).unwrap();
        let result = updated_params.to_vec().unwrap();
        let m_result = m.to_vec().unwrap();
        let v_result = v.to_vec().unwrap();
        
        // Should produce valid outputs
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(m_result.iter().all(|&x| x.is_finite()));
        assert!(v_result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adam_boundary() {
        let device = get_test_device().await;
        
        // Test with different learning rates
        let params1 = Tensor::from_data(
            &vec![1.0, 1.0, 1.0, 1.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let params2 = Tensor::from_data(
            &vec![1.0, 1.0, 1.0, 1.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &vec![0.1, 0.1, 0.1, 0.1],
            vec![4],
            device.clone(),
        ).unwrap();
        
        // Small learning rate
        let (updated1, _m, _v) = params1.adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None).unwrap();
        
        // Large learning rate
        let (updated2, _m, _v) = params2.adam_step(&gradients, 0.1, 0.9, 0.999, 1, None, None).unwrap();
        
        let result1 = updated1.to_vec().unwrap();
        let result2 = updated2.to_vec().unwrap();
        
        // Both should be valid and finite
        assert!(result1.iter().all(|&x| x.is_finite()));
        assert!(result2.iter().all(|&x| x.is_finite()));
        // Different learning rates should produce some updates
        assert!(result1.iter().zip([1.0, 1.0, 1.0, 1.0].iter()).any(|(a, b)| (a - b).abs() > 0.0));
        assert!(result2.iter().zip([1.0, 1.0, 1.0, 1.0].iter()).any(|(a, b)| (a - b).abs() > 0.0));
    }

    #[tokio::test]
    async fn test_adam_large_batch() {
        let device = get_test_device().await;
        
        // Larger parameter set (e.g., small neural network layer)
        let size = 128;
        let params_data: Vec<f32> = (0..size).map(|i| (i as f32) / 10.0).collect();
        let grads_data: Vec<f32> = vec![0.01; size];
        
        let params = Tensor::from_data(
            &params_data,
            vec![size],
            device.clone(),
        ).unwrap();
        
        let gradients = Tensor::from_data(
            &grads_data,
            vec![size],
            device.clone(),
        ).unwrap();
        
        let (updated_params, updated_m, updated_v) = params.adam_step(&gradients, 0.001, 0.9, 0.999, 1, None, None).unwrap();
        
        let result = updated_params.to_vec().unwrap();
        let m_result = updated_m.to_vec().unwrap();
        let v_result = updated_v.to_vec().unwrap();
        
        assert_eq!(result.len(), size);
        assert_eq!(m_result.len(), size);
        assert_eq!(v_result.len(), size);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adam_precision() {
        let device = get_test_device().await;
        
        // Test multiple steps with momentum accumulation
        let params = Tensor::from_data(
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
        let (params1, m1, v1) = params.adam_step(&gradients, 0.1, 0.9, 0.999, 1, None, None).unwrap();
        let result1 = params1.to_vec().unwrap();
        
        // Params should decrease (positive gradients, gradient descent)
        assert!(result1[0] < 10.0);
        assert!(result1[1] < 20.0);
        
        // Step 2 (with momentum)
        let (params2, _m2, _v2) = params1.adam_step(&gradients, 0.1, 0.9, 0.999, 2, Some(&m1), Some(&v1)).unwrap();
        let result2 = params2.to_vec().unwrap();
        
        // Should produce valid output
        assert!(result2.iter().all(|&x| x.is_finite()));
        // Should still be decreasing (both are less than original)
        assert!(result2[0] < 10.0);
        assert!(result2[1] < 20.0);
    }
}
