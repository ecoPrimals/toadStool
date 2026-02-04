//! Masked Select - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! NOTE: This implementation requires prefix sum computation.
//! For now, we compute prefix sum on CPU. A full GPU implementation
//! would require a parallel scan operation.

use crate::error::Result;
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;


/// Masked select operation
pub struct MaskedSelect {
    input: Tensor,
    mask: Tensor,
}

impl MaskedSelect {
    /// Create a new masked select operation
    pub fn new(input: Tensor, mask: Tensor) -> Result<Self> {
        if input.shape() != mask.shape() {
            return Err(crate::error::BarracudaError::ShapeMismatch {
                expected: input.shape().to_vec(),
                actual: mask.shape().to_vec(),
            });
        }

        Ok(Self {
            input,
            mask,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/masked_select.wgsl")
    }

    fn prefix_sum_shader() -> &'static str {
        include_str!("../shaders/prefix_sum.wgsl")
    }

    fn mask_convert_shader() -> &'static str {
        include_str!("../shaders/mask_convert.wgsl")
    }
    /// Compute GPU prefix sum for boolean mask
    fn compute_prefix_sum_gpu(
        device: &Arc<crate::device::WgpuDevice>,
        mask_buffer: &wgpu::Buffer,
        size: usize,
    ) -> Result<wgpu::Buffer> {
        let prefix_sum_buffer = device.create_buffer_u32(size)?;
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
            pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(prefix_sum_buffer)
    }

    /// Convert f32 mask to u32 mask on GPU
    fn convert_mask_gpu(
        device: &Arc<crate::device::WgpuDevice>,
        input_mask_buffer: &wgpu::Buffer,
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
                    resource: input_mask_buffer.as_entire_binding(),
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

    /// Read only the last element of a u32 buffer
    fn read_buffer_u32_last(
        device: &Arc<crate::device::WgpuDevice>,
        buffer: &wgpu::Buffer,
        size: usize,
    ) -> Result<u32> {
        if size == 0 {
            return Ok(0);
        }
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
            .map_err(|e| crate::error::BarracudaError::gpu(format!("Failed to map buffer: {:?}", e)))?
            .map_err(|e| crate::error::BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result_data: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();

        Ok(result_data[0])
    }

    /// Execute the masked select operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_size: usize = self.input.shape().iter().product();

        // Step 1: Create boolean mask buffer on GPU and convert f32 mask to u32
        let mask_buffer = device.create_buffer_u32(input_size)?;
        Self::convert_mask_gpu(device, self.mask.buffer(), &mask_buffer, input_size)?;

        // Step 2: Compute prefix sum on GPU
        let prefix_sum_buffer = Self::compute_prefix_sum_gpu(
            device,
            &mask_buffer,
            input_size,
        )?;

        // Step 3: Read only the last element of prefix sum to get output size
        let output_size = Self::read_buffer_u32_last(device, &prefix_sum_buffer, input_size)? as usize;

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            input_size: u32,
            _pad1: u32,
            _pad2: u32,
            _pad3: u32,
        }

        let params = Params {
            input_size: input_size as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MaskedSelect Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("MaskedSelect Shader"));

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MaskedSelect Bind Group Layout"),
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
            ],
        });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MaskedSelect Bind Group"),
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: prefix_sum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MaskedSelect Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("MaskedSelect Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MaskedSelect Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MaskedSelect Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((input_size as u32 + 255) / 256, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Return tensor without reading back (zero-copy)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![output_size],
            device.clone(),
        ))
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
    async fn test_masked_select_basic() {
        let device = get_test_device().await;
        let input = Tensor::from_data(
            &[1.0, 2.0, 3.0, 4.0, 5.0],
            vec![5],
            device.clone(),
        ).unwrap();
        let mask = Tensor::from_data(
            &[1.0, 0.0, 1.0, 0.0, 1.0],
            vec![5],
            device.clone(),
        ).unwrap();
        
        let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &vec![3]);
    }

    #[tokio::test]
    async fn test_masked_select_all_true() {
        let device = get_test_device().await;
        let input = Tensor::from_data(
            &[1.0, 2.0, 3.0],
            vec![3],
            device.clone(),
        ).unwrap();
        let mask = Tensor::from_data(
            &[1.0, 1.0, 1.0],
            vec![3],
            device.clone(),
        ).unwrap();
        
        let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &vec![3]);
    }

    #[tokio::test]
    async fn test_masked_select_all_false() {
        let device = get_test_device().await;
        let input = Tensor::from_data(
            &[1.0, 2.0, 3.0],
            vec![3],
            device.clone(),
        ).unwrap();
        let mask = Tensor::from_data(
            &[0.0, 0.0, 0.0],
            vec![3],
            device.clone(),
        ).unwrap();
        
        let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &vec![0]);
    }

    #[tokio::test]
    async fn test_masked_select_shape_mismatch() {
        let device = get_test_device().await;
        let input = Tensor::from_data(
            &[1.0, 2.0],
            vec![2],
            device.clone(),
        ).unwrap();
        let mask = Tensor::from_data(
            &[1.0, 1.0, 1.0],
            vec![3],
            device.clone(),
        ).unwrap();
        
        assert!(MaskedSelect::new(input, mask).is_err());
    }

    #[tokio::test]
    async fn test_masked_select_large() {
        let device = get_test_device().await;
        let data: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let input = Tensor::from_data(
            &data,
            vec![1000],
            device.clone(),
        ).unwrap();
        let mask_data: Vec<f32> = (0..1000).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
        let mask = Tensor::from_data(
            &mask_data,
            vec![1000],
            device.clone(),
        ).unwrap();
        
        let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
        assert_eq!(result.shape(), &vec![500]);
    }
}
