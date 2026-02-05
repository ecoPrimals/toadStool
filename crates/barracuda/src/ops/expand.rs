//! Expand - Broadcast tensor to larger shape - Pure WGSL
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (GPU-optimized)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//!
//! ## Algorithm
//!
//! Broadcasting expands singleton dimensions to match target shape:
//! ```text
//! Input:  [1, 3] → Output: [4, 3]
//! Repeats the input across the first dimension
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

pub struct Expand {
    input: Tensor,
    target_shape: Vec<usize>,
}

impl Expand {
    pub fn new(input: Tensor, target_shape: Vec<usize>) -> Self {
        Self { input, target_shape }
    }
    
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/expand.wgsl")
    }
    
    /// Compute broadcasted input shape following NumPy broadcasting rules
    /// 
    /// Broadcasting rules:
    /// - Dimensions are compared right-to-left
    /// - Each dimension must either:
    ///   - Be equal
    ///   - One of them is 1
    ///   - One of them doesn't exist (implicitly 1)
    /// - Missing dimensions are added at the front with size 1
    fn compute_broadcast_shape(input_shape: &[usize], target_shape: &[usize]) -> Result<Vec<usize>> {
        let input_rank = input_shape.len();
        let target_rank = target_shape.len();
        
        // Pad input shape with 1s at the front if needed
        let mut broadcasted_input_shape = vec![1; target_rank];
        let offset = target_rank.saturating_sub(input_rank);
        for (i, &dim) in input_shape.iter().enumerate() {
            broadcasted_input_shape[offset + i] = dim;
        }
        
        // Validate broadcasting compatibility (right-to-left)
        for i in (0..target_rank).rev() {
            let input_dim = broadcasted_input_shape[i];
            let target_dim = target_shape[i];
            
            if input_dim != target_dim && input_dim != 1 && target_dim != 1 {
                return Err(BarracudaError::InvalidShape {
                    expected: target_shape.to_vec(),
                    actual: input_shape.to_vec(),
                });
            }
        }
        
        Ok(broadcasted_input_shape)
    }
    
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_shape = self.input.shape();
        let output_size: usize = self.target_shape.iter().product();
        
        // Validate broadcasting compatibility (NumPy-style)
        let broadcasted_input_shape = Self::compute_broadcast_shape(input_shape, &self.target_shape)?;
        
        // Compute strides for input (with broadcasting: stride=0 for dims of size 1)
        // Standard stride computation: stride[i] = product of dimensions after i
        // For broadcasting: if shape[i] == 1, stride[i] = 0
        let num_dims = self.target_shape.len();
        let mut input_strides = vec![0; num_dims];
        
        // Compute strides backwards
        // Start with last dimension
        if broadcasted_input_shape[num_dims - 1] != 1 {
            input_strides[num_dims - 1] = 1;
        }
        
        // For each dimension, compute stride as product of subsequent dimensions
        for i in (0..num_dims - 1).rev() {
            if broadcasted_input_shape[i] == 1 {
                // Broadcast dimension: stride is 0
                input_strides[i] = 0;
            } else {
                // Compute stride as product of all dimensions after this one
                let mut stride = 1u32;
                for j in (i + 1)..num_dims {
                    stride *= broadcasted_input_shape[j] as u32;
                }
                input_strides[i] = stride as usize;
            }
        }
        
        // Compute output strides
        let mut output_strides = vec![1; num_dims];
        for i in (0..num_dims - 1).rev() {
            output_strides[i] = output_strides[i + 1] * self.target_shape[i + 1];
        }
        
        let output_buffer = device.create_buffer_f32(output_size)?;
        
        // Create buffers for shapes and strides
        let input_shape_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Input Shape"),
            contents: bytemuck::cast_slice(&broadcasted_input_shape.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let output_shape_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Output Shape"),
            contents: bytemuck::cast_slice(&self.target_shape.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let input_strides_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Input Strides"),
            contents: bytemuck::cast_slice(&input_strides.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let output_strides_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Output Strides"),
            contents: bytemuck::cast_slice(&output_strides.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        // Create params buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            output_size: u32,
            num_dims: u32,
            _pad1: u32,
            _pad2: u32,
        }
        
        let params = Params {
            output_size: output_size as u32,
            num_dims: num_dims as u32,
            _pad1: 0,
            _pad2: 0,
        };
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Bind group layout (7 bindings: params, input, input_shape, output_shape, input_strides, output_strides, output)
        let bind_group_layout = device.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Expand BGL"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                ],
            }
        );
        
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Expand BG"),
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
                    resource: input_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: input_strides_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: output_strides_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });
        
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Expand"));
        let pipeline_layout = device.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Expand PL"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }
        );
        
        let pipeline = device.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("Expand Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            }
        );
        
        let mut encoder = device.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Expand Encoder"),
            }
        );
        
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Expand Pass"),
                timestamp_writes: None,
            });
            
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = (output_size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        device.queue.submit(Some(encoder.finish()));
        
        Ok(Tensor::from_buffer(
            output_buffer,
            self.target_shape.clone(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Expand/broadcast tensor to target shape
    pub fn expand_wgsl(self, target_shape: Vec<usize>) -> Result<Self> {
        Expand::new(self, target_shape).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_expand_basic() {
        let device = get_test_device().await;
        // Broadcast from [3] to [9] (repeat 3 times)
        let input_data = vec![1.0, 2.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device)
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![9]).unwrap().to_vec().unwrap();
        
        assert_eq!(result.len(), 9);
        // Should repeat pattern: [1,2,3,1,2,3,1,2,3]
        let expected = vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0];
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6, "Expected {}, got {}", e, r);
        }
    }

    #[tokio::test]
    async fn test_expand_single_element() {
        let device = get_test_device().await;
        // Broadcast single value to multiple
        let input_data = vec![5.0];
        let input = Tensor::from_vec_on(input_data, vec![1], device)
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![10]).unwrap().to_vec().unwrap();
        
        assert_eq!(result.len(), 10);
        assert!(result.iter().all(|&x| (x - 5.0).abs() < 1e-6));
    }

    #[tokio::test]
    async fn test_expand_no_change() {
        let device = get_test_device().await;
        // No expansion needed (already target size)
        let input_data = vec![1.0, 2.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone())
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![3]).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output, input_data);
    }

    #[tokio::test]
    async fn test_expand_boundary() {
        let dev = get_test_device().await;

        // Large expansion factor
        let input = vec![3.14];
        let output = Tensor::from_vec_on(input, vec![1], dev.clone())
            .await
            .unwrap()
            .expand_wgsl(vec![1000])
            .unwrap()
            .to_vec()
            .unwrap();
        assert_eq!(output.len(), 1000);
        assert!(output.iter().all(|&x| (x - 3.14).abs() < 1e-5));

        // Smaller expansion
        let input = vec![99.0];
        let output = Tensor::from_vec_on(input, vec![1], dev.clone())
            .await
            .unwrap()
            .expand_wgsl(vec![5])
            .unwrap()
            .to_vec()
            .unwrap();
        assert_eq!(output.len(), 5);
        assert!(output.iter().all(|&x| (x - 99.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_expand_large_batch() {
        let dev = get_test_device().await;

        // Single value to large tensor
        let input = vec![42.0];
        let output = Tensor::from_vec_on(input, vec![1], dev.clone())
            .await
            .unwrap()
            .expand_wgsl(vec![10000])
            .unwrap()
            .to_vec()
            .unwrap();
        assert_eq!(output.len(), 10000);
        assert!(output.iter().all(|&x| (x - 42.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_expand_precision() {
        let dev = get_test_device().await;

        // Verify exact value preserved during broadcast
        let input = vec![1.23456];
        let output = Tensor::from_vec_on(input, vec![1], dev.clone())
            .await
            .unwrap()
            .expand_wgsl(vec![100])
            .unwrap()
            .to_vec()
            .unwrap();
        assert_eq!(output.len(), 100);

        // All values should match exactly
        for val in output.iter() {
            assert!((val - 1.23456).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_expand_2d_broadcast_second_dim() {
        let dev = get_test_device().await;
        // (3, 1) → (3, 5): broadcast second dim
        let input_data: Vec<f32> = vec![1.0, 2.0, 3.0];
        let input = Tensor::from_vec_on(input_data, vec![3, 1], dev.clone())
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![3, 5]).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(result.shape(), &vec![3, 5]);
        // Each row should be the same: [1,1,1,1,1], [2,2,2,2,2], [3,3,3,3,3]
        for i in 0..3 {
            let expected_val = (i + 1) as f32;
            for j in 0..5 {
                let idx = i * 5 + j;
                assert!((output[idx] - expected_val).abs() < 1e-6,
                    "Expected {} at [{}, {}], got {}", expected_val, i, j, output[idx]);
            }
        }
    }

    #[tokio::test]
    async fn test_expand_2d_broadcast_first_dim() {
        let dev = get_test_device().await;
        // (1, 5) → (4, 5): broadcast first dim
        let input_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![1, 5], dev.clone())
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![4, 5]).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(result.shape(), &vec![4, 5]);
        // All rows should be the same: [1,2,3,4,5]
        for i in 0..4 {
            for j in 0..5 {
                let idx = i * 5 + j;
                assert!((output[idx] - input_data[j]).abs() < 1e-6,
                    "Expected {} at [{}, {}], got {}", input_data[j], i, j, output[idx]);
            }
        }
    }

    #[tokio::test]
    async fn test_expand_add_dimension() {
        let dev = get_test_device().await;
        // (3,) → (3, 5): add dimension then broadcast
        let input_data: Vec<f32> = vec![1.0, 2.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], dev.clone())
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![3, 5]).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(result.shape(), &vec![3, 5]);
        // Each row should repeat the same value: [1,1,1,1,1], [2,2,2,2,2], [3,3,3,3,3]
        for i in 0..3 {
            let expected_val = input_data[i];
            for j in 0..5 {
                let idx = i * 5 + j;
                assert!((output[idx] - expected_val).abs() < 1e-6,
                    "Expected {} at [{}, {}], got {}", expected_val, i, j, output[idx]);
            }
        }
    }

    #[tokio::test]
    async fn test_expand_3d_broadcast_middle_dim() {
        let dev = get_test_device().await;
        // (3, 1, 5) → (3, 4, 5): broadcast middle dim
        let input_data: Vec<f32> = (0..15).map(|i| i as f32).collect(); // 3*1*5 = 15
        let input = Tensor::from_vec_on(input_data, vec![3, 1, 5], dev.clone())
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![3, 4, 5]).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(result.shape(), &vec![3, 4, 5]);
        // For each of the 3 slices, the middle dimension should be broadcasted
        // Slice 0: values 0-4 repeated 4 times
        // Slice 1: values 5-9 repeated 4 times
        // Slice 2: values 10-14 repeated 4 times
        for i in 0..3 {
            for j in 0..4 {
                for k in 0..5 {
                    let idx = i * 20 + j * 5 + k;
                    let expected_val = (i * 5 + k) as f32;
                    assert!((output[idx] - expected_val).abs() < 1e-6,
                        "Expected {} at [{}, {}, {}], got {}", expected_val, i, j, k, output[idx]);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_expand_scalar_to_tensor() {
        let dev = get_test_device().await;
        // Scalar (1,) → (2, 3, 4)
        let input_data = vec![42.0];
        let input = Tensor::from_vec_on(input_data, vec![1], dev.clone())
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![2, 3, 4]).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(result.shape(), &vec![2, 3, 4]);
        assert_eq!(output.len(), 24);
        assert!(output.iter().all(|&x| (x - 42.0).abs() < 1e-6));
    }

    #[tokio::test]
    async fn test_expand_incompatible_shapes() {
        let dev = get_test_device().await;
        // (3, 4) cannot broadcast to (3, 5) - both dims are > 1 and different
        let input_data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data, vec![3, 4], dev.clone())
            .await
            .unwrap();
        
        assert!(input.expand_wgsl(vec![3, 5]).is_err());
    }

    #[tokio::test]
    async fn test_expand_4d_broadcast() {
        let dev = get_test_device().await;
        // (1, 3, 1, 5) → (2, 3, 4, 5): broadcast first and third dims
        let input_data: Vec<f32> = (0..15).map(|i| i as f32).collect(); // 1*3*1*5 = 15
        let input = Tensor::from_vec_on(input_data, vec![1, 3, 1, 5], dev.clone())
            .await
            .unwrap();
        let result = input.expand_wgsl(vec![2, 3, 4, 5]).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(result.shape(), &vec![2, 3, 4, 5]);
        // Verify broadcasting: first and third dims are broadcasted
        for i in 0..2 {
            for j in 0..3 {
                for k in 0..4 {
                    for l in 0..5 {
                        let idx = i * 60 + j * 20 + k * 5 + l;
                        let expected_val = (j * 5 + l) as f32;
                        assert!((output[idx] - expected_val).abs() < 1e-6,
                            "Expected {} at [{}, {}, {}, {}], got {}", expected_val, i, j, k, l, output[idx]);
                    }
                }
            }
        }
    }
}
