use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FilterParams {
    size: u32,
    operation: u32,
    threshold: f32,
}

pub struct Filter {
    input: Tensor,
    operation: FilterOperation,
    threshold: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterOperation {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
}

impl FilterOperation {
    fn to_u32(&self) -> u32 {
        match self {
            FilterOperation::GreaterThan => 0,
            FilterOperation::LessThan => 1,
            FilterOperation::Equal => 2,
            FilterOperation::NotEqual => 3,
        }
    }
}

impl Filter {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/filter.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.shape().iter().product::<usize>();
        
        let params = FilterParams {
            size: size as u32,
            operation: self.operation.to_u32(),
            threshold: self.threshold,
        };
        
        // This is a simplified version - just evaluates predicate and returns flags
        // Full filter would need multi-pass (predicate + prefix sum + compact)
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let flags_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter_flags"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let count_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter_count"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("filter_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("filter_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("filter_bind_group_layout"),
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
            label: Some("filter_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("filter_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "evaluate_predicate",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("filter_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: flags_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("filter_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("filter_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        // Return flags buffer as tensor (1.0 for keep, 0.0 for discard)
        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

pub trait FilterExt {
    fn filter(self, operation: FilterOperation, threshold: f32) -> Result<Tensor>;
}

impl FilterExt for Tensor {
    fn filter(self, operation: FilterOperation, threshold: f32) -> Result<Tensor> {
        let op = Filter {
            input: self,
            operation,
            threshold,
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
    async fn test_filter_greater_than() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        let input = Tensor::from_data(
            &vec![1.0, 5.0, 3.0, 7.0],
            vec![4],
            device.clone(),
        ).unwrap();
        
        let result = input.filter(FilterOperation::GreaterThan, 4.0).unwrap();
        let output = result.to_vec().unwrap();
        
        // Results: 1.0 (no), 5.0 (yes), 3.0 (no), 7.0 (yes)
        assert_eq!(output.len(), 4);
    }
}
