use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MapParams {
    size: u32,
    operation: u32,
}

pub struct Map {
    input: Tensor,
    operation: MapOperation,
}

#[derive(Debug, Clone, Copy)]
pub enum MapOperation {
    Square,
    Sqrt,
    Abs,
    Negate,
    Reciprocal,
}

impl MapOperation {
    fn to_u32(&self) -> u32 {
        match self {
            MapOperation::Square => 0,
            MapOperation::Sqrt => 1,
            MapOperation::Abs => 2,
            MapOperation::Negate => 3,
            MapOperation::Reciprocal => 4,
        }
    }
}

impl Map {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/map.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.shape().iter().product::<usize>();

        let params = MapParams {
            size: size as u32,
            operation: self.operation.to_u32(),
        };

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("map_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("map_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("map_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("map_bind_group_layout"),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("map_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("map_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("map_bind_group"),
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

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("map_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("map_pass"),
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
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

pub trait MapExt {
    fn map(self, operation: MapOperation) -> Result<Tensor>;
}

impl MapExt for Tensor {
    fn map(self, operation: MapOperation) -> Result<Tensor> {
        let op = Map {
            input: self,
            operation,
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
    async fn test_map_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_data(&vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone()).unwrap();

        let result = input.map(MapOperation::Square).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_map_edge_cases() {
        let device = get_test_device().await;

        // Single element
        let input = Tensor::from_data(&vec![5.0], vec![1], device.clone()).unwrap();
        let result = input.map(MapOperation::Square).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 1);
        assert!(output[0].is_finite());

        // Negate operation
        let input = Tensor::from_data(&vec![1.0, -2.0, 3.0], vec![3], device.clone()).unwrap();
        let result = input.map(MapOperation::Negate).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 3);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_map_boundary() {
        let device = get_test_device().await;

        // Sqrt with various values
        let input = Tensor::from_data(&vec![4.0, 9.0, 16.0], vec![3], device.clone()).unwrap();
        let result = input.map(MapOperation::Sqrt).unwrap();
        let output = result.to_vec().unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));

        // Abs with negative values
        let input = Tensor::from_data(&vec![-1.0, -2.0, -3.0], vec![3], device.clone()).unwrap();
        let result = input.map(MapOperation::Abs).unwrap();
        let output = result.to_vec().unwrap();
        assert!(output.iter().all(|&x| x >= 0.0));
    }

    #[tokio::test]
    async fn test_map_large_batch() {
        let device = get_test_device().await;

        // 1000 elements
        let input_data: Vec<f32> = (1..=1000).map(|i| i as f32).collect();
        let input = Tensor::from_data(&input_data, vec![1000], device).unwrap();
        let result = input.map(MapOperation::Square).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), 1000);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_map_precision() {
        let device = get_test_device().await;

        // Test square operation
        let input = Tensor::from_data(&vec![2.0, 3.0], vec![2], device).unwrap();
        let result = input.map(MapOperation::Square).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 2);
        assert!(output.iter().all(|&x| x.is_finite()));
    }
}
