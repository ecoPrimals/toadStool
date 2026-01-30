use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DotProductParams {
    size: u32,
}

pub struct DotProduct {
    a: Tensor,
    b: Tensor,
}

impl DotProduct {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/dotproduct.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.a.device();
        let size = self.a.shape().iter().product::<usize>();
        
        let params = DotProductParams {
            size: size as u32,
        };
        
        let num_workgroups = ((size + 255) / 256) as u32;
        
        // Partial sums buffer
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dotproduct_output"),
            size: (num_workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dotproduct_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dotproduct_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dotproduct_bind_group_layout"),
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
            label: Some("dotproduct_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("dotproduct_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dotproduct_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.b.buffer().as_entire_binding(),
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
            label: Some("dotproduct_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("dotproduct_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        // Return partial sums (caller can sum them for final result)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![num_workgroups as usize],
            device.clone(),
        ))
    }
}

pub trait DotProductExt {
    fn dotproduct(self, b: &Tensor) -> Result<Tensor>;
}

impl DotProductExt for Tensor {
    fn dotproduct(self, b: &Tensor) -> Result<Tensor> {
        let op = DotProduct {
            a: self,
            b: b.clone(),
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
    async fn test_dotproduct() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        let a = Tensor::from_data(
            &vec![1.0, 2.0, 3.0, 4.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let b = Tensor::from_data(
            &vec![1.0, 1.0, 1.0, 1.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let result = a.dotproduct(&b).unwrap();
        let partial_sums = result.to_vec().unwrap();
        
        // Sum partial results
        let total: f32 = partial_sums.iter().sum();
        
        // 1*1 + 2*1 + 3*1 + 4*1 = 10
        assert!((total - 10.0).abs() < 1e-4);
    }
}
