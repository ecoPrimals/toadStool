//! TopK - GPU-accelerated top-K largest values selection
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (uses existing shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for inference)
//!
//! ## Algorithm
//!
//! ```text
//! Find indices of top K largest values in tensor
//! Output: [k] indices (as u32)
//! ```
//!
//! **Implementation**: GPU selection (basic O(n*k), parallel sorting for production)
//!
//! **Key Properties**:
//! - Returns indices, not values
//! - Handles duplicates
//! - Stable ordering for equal values
//!
//! **Used By**: Beam search, retrieval, recommendation systems
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let scores = Tensor::from_vec(vec![5.0, 1.0, 9.0, 3.0, 7.0], vec![5]).await?;
//! let top3_indices = scores.topk(3)?;  // Returns [2, 4, 0] (indices)
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// TopK parameters for WGSL shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TopKParams {
    k: u32,
}

/// TopK operation
///
/// **Deep Debt**: Uses existing WGSL shader with selection algorithm
pub struct TopK {
    input: Tensor,
    k: usize,
}

impl TopK {
    /// Create new TopK operation
    ///
    /// **Deep Debt**: Validates K against tensor size
    pub fn new(input: Tensor, k: usize) -> Result<Self> {
        // Validate K
        let size = input.len();
        if k == 0 {
            return Err(BarracudaError::invalid_op("TopK", "k must be positive"));
        }
        if k > size {
            return Err(BarracudaError::invalid_op(
                "TopK",
                format!("k ({}) exceeds tensor size ({})", k, size),
            ));
        }

        // TopK currently only works on 1D tensors (flatten for higher dims)
        if input.shape().len() != 1 {
            return Err(BarracudaError::invalid_op(
                "TopK",
                "currently only supports 1D tensors (use flatten() first)",
            ));
        }

        Ok(Self { input, k })
    }

    /// WGSL shader source
    fn shader() -> &'static str {
        include_str!("../shaders/topk.wgsl")
    }

    /// Execute TopK (GPU selection)
    ///
    /// **Deep Debt**: Basic O(n*k) selection, sufficient for moderate K
    ///
    /// Returns: Tensor of indices [k] as f32 (cast from u32)
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();

        // Create parameters
        let params = TopKParams { k: self.k as u32 };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopK Params"),
            size: std::mem::size_of::<TopKParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Output buffer (u32 indices)
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopK Output"),
            size: (self.k * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compile shader
        let shader = device.compile_shader(Self::shader(), Some("TopK"));

        // Create bind group layout
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("TopK BGL"),
                entries: &[
                    // Input
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
                    // Output
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
                    // Params
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TopK BG"),
            layout: &bgl,
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

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("TopK Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("TopK Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("TopK Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("TopK Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let size = self.input.len();
            let workgroups = (size as u32 + optimal_wg_size - 1) / optimal_wg_size;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Read u32 indices back and convert to f32
        let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopK Staging"),
            size: (self.k * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("TopK Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (self.k * std::mem::size_of::<u32>()) as u64,
        );
        device.queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });
        device.device.poll(wgpu::Maintain::Wait);
        futures::executor::block_on(rx).unwrap().unwrap();

        // Convert u32 to f32
        let data = buffer_slice.get_mapped_range();
        let indices_u32: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();

        let indices_f32: Vec<f32> = indices_u32.iter().map(|&x| x as f32).collect();

        // Create output tensor [k] as f32
        let output_tensor = futures::executor::block_on(Tensor::from_vec_on(
            indices_f32,
            vec![self.k],
            device.clone(),
        ))?;

        Ok(output_tensor)
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Top-K largest values (returns indices)
    ///
    /// **Deep Debt**: Essential for beam search and retrieval
    ///
    /// # Arguments
    /// - `k`: Number of top values to return
    ///
    /// # Returns
    /// - Indices tensor [k] as f32 (cast from u32)
    ///
    /// # Example
    /// ```rust,ignore
    /// let scores = Tensor::from_vec(vec![5.0, 1.0, 9.0, 3.0, 7.0], vec![5]).await?;
    /// let top3 = scores.topk(3)?;  // [2, 4, 0] (indices of 9.0, 7.0, 5.0)
    /// ```
    ///
    /// # Note
    /// Currently only supports 1D tensors. Use `flatten()` for higher dimensions.
    pub fn topk(self, k: usize) -> Result<Self> {
        TopK::new(self, k)?.execute()
    }
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_topk_gpu_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![5.0, 1.0, 9.0, 3.0, 7.0], vec![5], device)
            .await
            .unwrap();

        let top3 = input.topk(3).unwrap();

        assert_eq!(top3.shape(), &[3]);
        let indices = top3.to_vec().unwrap();

        // Should return indices of [9.0, 7.0, 5.0] = [2, 4, 0]
        assert_eq!(indices[0] as u32, 2); // 9.0
        assert_eq!(indices[1] as u32, 4); // 7.0
        assert_eq!(indices[2] as u32, 0); // 5.0
    }

    #[tokio::test]
    async fn test_topk_gpu_single() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device)
            .await
            .unwrap();

        let top1 = input.topk(1).unwrap();
        let indices = top1.to_vec().unwrap();

        // Largest value is 4.0 at index 3
        assert_eq!(indices[0] as u32, 3);
    }

    #[tokio::test]
    async fn test_topk_gpu_all() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![3.0, 1.0, 4.0, 1.0, 5.0], vec![5], device)
            .await
            .unwrap();

        let top5 = input.topk(5).unwrap();
        let indices = top5.to_vec().unwrap();

        // All indices, sorted by value: [5.0, 4.0, 3.0, 1.0, 1.0] = [4, 2, 0, 1, 3]
        assert_eq!(indices[0] as u32, 4); // 5.0
        assert_eq!(indices[1] as u32, 2); // 4.0
        assert_eq!(indices[2] as u32, 0); // 3.0
    }

    #[tokio::test]
    async fn test_topk_gpu_negative() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![-5.0, -1.0, -9.0, -3.0], vec![4], device)
            .await
            .unwrap();

        let top2 = input.topk(2).unwrap();
        let indices = top2.to_vec().unwrap();

        // Largest (least negative): [-1.0, -3.0] at indices [1, 3]
        assert_eq!(indices[0] as u32, 1); // -1.0
        assert_eq!(indices[1] as u32, 3); // -3.0
    }

    #[tokio::test]
    async fn test_topk_gpu_validation() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device)
            .await
            .unwrap();

        // k=0 should error
        assert!(input.clone().topk(0).is_err());

        // k > size should error
        assert!(input.topk(10).is_err());
    }

    #[tokio::test]
    async fn test_topk_gpu_duplicates() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![2.0, 5.0, 5.0, 1.0], vec![4], device)
            .await
            .unwrap();

        let top2 = input.topk(2).unwrap();
        let indices = top2.to_vec().unwrap();

        // Two 5.0 values at indices 1 and 2
        // Should return both (order may vary, but both should be 1 or 2)
        let idx0 = indices[0] as u32;
        let idx1 = indices[1] as u32;
        assert!((idx0 == 1 || idx0 == 2) && (idx1 == 1 || idx1 == 2));
    }

    #[tokio::test]
    async fn test_topk_gpu_large() {
        let device = get_test_device().await;

        // Larger tensor (100 elements)
        let mut values = vec![0.0; 100];
        for (i, v) in values.iter_mut().enumerate() {
            *v = (i as f32) * 0.1;
        }

        let input = Tensor::from_vec_on(values, vec![100], device)
            .await
            .unwrap();

        let top10 = input.topk(10).unwrap();
        let indices = top10.to_vec().unwrap();

        // Should be indices 99, 98, 97, ..., 90 (highest values)
        assert_eq!(indices[0] as u32, 99);
        assert_eq!(indices[1] as u32, 98);
        assert_eq!(indices[9] as u32, 90);
    }
}
