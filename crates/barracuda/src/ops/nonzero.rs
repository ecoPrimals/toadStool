//! NonZero - GPU prefix sum implementation
//!
//! **Deep Debt Principles**:
//! - Complete GPU implementation: Uses prefix_sum.wgsl for GPU parallel scan
//! - No CPU fallbacks: All computation on GPU
//! - Self-knowledge: Validates inputs
//! - Modern idiomatic Rust: Result<T, E>, no unwrap()

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct NonZeroParams {
    input_size: u32,
    _padding: [u32; 3],
}

pub struct NonZero {
    input: Tensor,
}

impl NonZero {
    pub fn new(input: Tensor) -> Result<Self> {
        if input.is_empty() {
            return Err(BarracudaError::invalid_op(
                "nonzero",
                "Cannot find nonzero elements in empty tensor",
            ));
        }

        Ok(Self { input })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/nonzero.wgsl")
    }

    fn prefix_sum_shader() -> &'static str {
        include_str!("../shaders/prefix_sum.wgsl")
    }

    fn mask_convert_shader() -> &'static str {
        include_str!("../shaders/mask_convert.wgsl")
    }

    fn u32_to_f32_shader() -> &'static str {
        include_str!("../shaders/u32_to_f32.wgsl")
    }

    /// Read u32 buffer from GPU
    #[allow(dead_code)]
    fn read_buffer_u32(
        device: &Arc<crate::device::WgpuDevice>,
        buffer: &wgpu::Buffer,
        size: usize,
    ) -> Result<Vec<u32>> {
        let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer U32"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Read Buffer Encoder"),
        });
        encoder.copy_buffer_to_buffer(
            buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<u32>()) as u64,
        );
        device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device.device.poll(wgpu::Maintain::Wait);

        let _result = futures::executor::block_on(receiver)
            .map_err(|e| BarracudaError::gpu(format!("Failed to map buffer: {:?}", e)))?
            .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result_data: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();

        Ok(result_data)
    }

    /// Compute GPU prefix sum for boolean mask
    fn compute_prefix_sum_gpu(
        device: &Arc<crate::device::WgpuDevice>,
        mask_buffer: &wgpu::Buffer,
        size: usize,
    ) -> Result<wgpu::Buffer> {
        // Create output buffer for prefix sum
        let prefix_sum_buffer = device.create_buffer_u32(size)?;
        
        // Create scratch buffer (required by prefix_sum shader)
        let scratch_buffer = device.create_buffer_u32(size)?;

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct PrefixSumParams {
            size: u32,
            _pad1: u32,
            _pad2: u32,
            _pad3: u32,
        }

        let params = PrefixSumParams {
            size: size as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PrefixSum Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PrefixSum Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PrefixSum Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mask_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: prefix_sum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: scratch_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::prefix_sum_shader(), Some("PrefixSum"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PrefixSum Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PrefixSum Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "inclusive_scan",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("PrefixSum Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PrefixSum Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1); // Sequential scan uses single workgroup
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(prefix_sum_buffer)
    }

    /// Convert f32 mask to u32 mask on GPU
    fn convert_mask_gpu(
        device: &Arc<crate::device::WgpuDevice>,
        input_buffer: &wgpu::Buffer,
        mask_buffer: &wgpu::Buffer,
        size: usize,
    ) -> Result<()> {
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MaskParams {
            size: u32,
            _pad1: u32,
            _pad2: u32,
            _pad3: u32,
        }

        let params = MaskParams {
            size: size as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mask Convert Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mask Convert Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
            ],
        });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mask Convert Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mask_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::mask_convert_shader(), Some("Mask Convert"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mask Convert Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mask Convert Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mask Convert Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Mask Convert Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    /// Convert u32 indices to f32 on GPU
    fn convert_u32_to_f32_gpu(
        device: &Arc<crate::device::WgpuDevice>,
        u32_buffer: &wgpu::Buffer,
        f32_buffer: &wgpu::Buffer,
        size: usize,
    ) -> Result<()> {
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ConvertParams {
            size: u32,
            _pad1: u32,
            _pad2: u32,
            _pad3: u32,
        }

        let params = ConvertParams {
            size: size as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("U32 to F32 Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("U32 to F32 Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
            ],
        });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("U32 to F32 Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: u32_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: f32_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::u32_to_f32_shader(), Some("U32 to F32"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("U32 to F32 Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("U32 to F32 Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("U32 to F32 Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("U32 to F32 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    /// Read only the last element of a u32 buffer (for getting prefix sum total)
    fn read_buffer_u32_last(
        device: &Arc<crate::device::WgpuDevice>,
        buffer: &wgpu::Buffer,
        size: usize,
    ) -> Result<u32> {
        if size == 0 {
            return Ok(0);
        }
        // Read only the last element
        let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer U32 Last"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Read Buffer Last Encoder"),
        });
        encoder.copy_buffer_to_buffer(
            buffer,
            ((size - 1) * std::mem::size_of::<u32>()) as u64,
            &staging_buffer,
            0,
            std::mem::size_of::<u32>() as u64,
        );
        device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device.device.poll(wgpu::Maintain::Wait);

        let _result = futures::executor::block_on(receiver)
            .map_err(|e| BarracudaError::gpu(format!("Failed to map buffer: {:?}", e)))?
            .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result_data: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();

        Ok(result_data[0])
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_size: usize = self.input.len();

        // Step 1: Create boolean mask on GPU (convert f32 input to u32 mask)
        let mask_buffer = device.create_buffer_u32(input_size)?;
        Self::convert_mask_gpu(device, self.input.buffer(), &mask_buffer, input_size)?;

        // Step 2: Compute prefix sum on GPU
        let prefix_sum_buffer = Self::compute_prefix_sum_gpu(device, &mask_buffer, input_size)?;

        // Step 3: Read only the last element of prefix sum to get output size
        let output_size = Self::read_buffer_u32_last(device, &prefix_sum_buffer, input_size)? as usize;

        if output_size == 0 {
            // No nonzero elements
            return Ok(Tensor::from_buffer(
                device.create_buffer_u32(0)?,
                vec![0],
                device.clone(),
            ));
        }

        // Step 4: Execute nonzero shader to compact indices
        let output_buffer = device.create_buffer_u32(output_size)?;

        let params = NonZeroParams {
            input_size: input_size as u32,
            _padding: [0; 3],
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NonZero Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("NonZero Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
            ],
        });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NonZero Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: prefix_sum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("NonZero"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NonZero Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NonZero Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("NonZero Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NonZero Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (input_size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Step 5: Convert u32 indices to f32 on GPU (for Tensor compatibility)
        let indices_f32_buffer = device.create_buffer_f32(output_size)?;
        Self::convert_u32_to_f32_gpu(device, &output_buffer, &indices_f32_buffer, output_size)?;

        Ok(Tensor::from_buffer(
            indices_f32_buffer,
            vec![output_size],
            device.clone(),
        ))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_nonzero_basic() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![0.0, 1.0, 0.0, 2.0, 0.0], vec![5], device.clone())
            .await
            .unwrap();
        
        let result = NonZero::new(input).unwrap().execute().unwrap();
        let indices = result.to_vec().unwrap();
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0] as u32, 1);
        assert_eq!(indices[1] as u32, 3);
    }

    #[tokio::test]
    async fn test_nonzero_all_zero() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![0.0, 0.0, 0.0], vec![3], device.clone())
            .await
            .unwrap();
        
        let result = NonZero::new(input).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &[0]);
    }

    #[tokio::test]
    async fn test_nonzero_all_nonzero() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        
        let result = NonZero::new(input).unwrap().execute().unwrap();
        let indices = result.to_vec().unwrap();
        assert_eq!(indices.len(), 3);
    }

    #[tokio::test]
    async fn test_nonzero_2d() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(
            vec![0.0, 1.0, 0.0, 2.0, 0.0, 3.0],
            vec![2, 3],
            device.clone(),
        )
        .await
        .unwrap();
        
        let result = NonZero::new(input).unwrap().execute().unwrap();
        let indices = result.to_vec().unwrap();
        assert_eq!(indices.len(), 3);
    }

    #[tokio::test]
    async fn test_nonzero_empty() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![], vec![0], device.clone())
            .await
            .unwrap();
        
        assert!(NonZero::new(input).is_err());
    }
}
