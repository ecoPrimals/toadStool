use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalMaxPoolParams {
    batch_size: u32,
    channels: u32,
    height: u32,
    width: u32,
}

pub struct GlobalMaxPool {
    input: Tensor,
}

impl GlobalMaxPool {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/global_maxpool.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        
        if shape.len() != 4 {
            return Err(crate::error::BarracudaError::invalid_op("Shape Error", 
                format!("GlobalMaxPool expects 4D input [batch, channels, height, width], got shape {:?}", shape)
            ));
        }
        
        let batch_size = shape[0];
        let channels = shape[1];
        let height = shape[2];
        let width = shape[3];
        
        let params = GlobalMaxPoolParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            height: height as u32,
            width: width as u32,
        };
        
        let output_shape = &vec![batch_size, channels, 1, 1];
        let output_size = output_shape.iter().product::<usize>() * std::mem::size_of::<f32>();
        
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("global_maxpool_output"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("global_maxpool_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("global_maxpool_shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });
        
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("global_maxpool_bind_group_layout"),
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
        
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("global_maxpool_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("global_maxpool_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("global_maxpool_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("global_maxpool_encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("global_maxpool_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let num_outputs = (batch_size * channels) as u32;
            let workgroups = ((num_outputs + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape.to_vec(),
            device.clone(),
        ))
    }
}

pub trait GlobalMaxPoolExt {
    fn global_maxpool(self) -> Result<Tensor>;
}

impl GlobalMaxPoolExt for Tensor {
    fn global_maxpool(self) -> Result<Tensor> {
        let op = GlobalMaxPool { input: self };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_global_maxpool() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        
        let input = Tensor::from_data(
            &vec![
                // Batch 0, Channel 0
                1.0, 2.0, 3.0, 4.0,
                5.0, 6.0, 7.0, 8.0,
                // Batch 0, Channel 1
                9.0, 10.0, 11.0, 12.0,
                13.0, 14.0, 15.0, 16.0,
            ],
            vec![1, 2, 2, 4],
            device.clone(),
        ).unwrap();
        
        let result = input.global_maxpool().unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(result.shape(), &[1, 2, 1, 1]);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], 8.0);  // Max of first channel
        assert_eq!(output[1], 16.0); // Max of second channel
    }
}
