//! Element-wise addition
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (universal compute)
//! - ✅ Capability-based dispatch (vendor-optimized)
//! - ✅ Vendor-specific workgroup sizes (NVIDIA: 64, AMD: 128)
//! - ✅ Pipeline caching (compile once, dispatch many)
//! - ✅ Buffer pooling (zero allocation after warmup)
//!
//! Formula: C = A + B (element-wise)

use crate::device::pipeline_cache::{BindGroupLayoutSignature, GLOBAL_CACHE};
use crate::device::tensor_context::get_device_context;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Shader source optimized for NVIDIA GPUs (WG=64)
const SHADER_WG64: &str = include_str!("../shaders/math/elementwise_add_wg64.wgsl");

/// Shader source optimized for AMD GPUs (WG=128)  
const SHADER_WG128: &str = include_str!("../shaders/math/elementwise_add_wg128.wgsl");

/// Default shader (WG=256, fallback)
const SHADER_DEFAULT: &str = include_str!("../shaders/math/elementwise_add.wgsl");

/// Element-wise addition operation
pub struct Add {
    lhs: Tensor,
    rhs: Tensor,
}

impl Add {
    /// Create Add operation
    pub fn new(lhs: Tensor, rhs: Tensor) -> Result<Self> {
        // Verify shapes match
        if lhs.shape() != rhs.shape() {
            return Err(BarracudaError::shape_mismatch(
                lhs.shape().to_vec(),
                rhs.shape().to_vec(),
            ));
        }
        Ok(Self { lhs, rhs })
    }

    /// Select vendor-optimized shader based on GPU and tensor size
    /// 
    /// Benchmarks show:
    /// - NVIDIA: WG=64 is 3x faster than WG=256
    /// - AMD: WG=128 is 2x faster than WG=64
    /// 
    /// Note: wgpu limits dispatch to 65535 workgroups per dimension,
    /// so for very large tensors we use larger workgroup sizes.
    fn wgsl_shader(device_name: &str, size: usize) -> (&'static str, u32) {
        let lower = device_name.to_lowercase();
        
        // Calculate workgroup size based on tensor size to stay within dispatch limits
        let max_dispatch = 65535u32;
        
        // Optimal sizes from benchmarks
        let (nvidia_wg, amd_wg) = (64u32, 128u32);
        
        // For NVIDIA
        if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("rtx") || lower.contains("gtx") {
            let needed_workgroups = (size as u32).div_ceil(nvidia_wg);
            if needed_workgroups <= max_dispatch {
                (SHADER_WG64, nvidia_wg)
            } else {
                // Fall back to larger workgroup for huge tensors
                (SHADER_DEFAULT, 256)
            }
        } else if lower.contains("amd") || lower.contains("radeon") || lower.contains("radv") {
            let needed_workgroups = (size as u32).div_ceil(amd_wg);
            if needed_workgroups <= max_dispatch {
                (SHADER_WG128, amd_wg)
            } else {
                (SHADER_DEFAULT, 256)
            }
        } else {
            (SHADER_DEFAULT, 256)
        }
    }

    /// Execute addition on tensors
    ///
    /// Uses cached shader and pipeline for fast repeated calls.
    /// First call compiles/caches, subsequent calls reuse.
    /// Output buffer is acquired from pool for zero-allocation steady-state.
    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size = self.lhs.len();

        // Get device context for buffer pooling
        let ctx = get_device_context(device);

        // Select vendor-optimized shader based on GPU and tensor size
        let device_name = device.name();
        let (shader_source, workgroup_size) = Self::wgsl_shader(device_name, size);

        // Acquire pooled output buffer (returns to pool when tensor dropped!)
        let output_buffer = ctx.acquire_pooled_output(size);

        // Get cached bind group layout (using adapter info for proper multi-GPU keying)
        let layout_sig = BindGroupLayoutSignature::elementwise_binary();
        let adapter_info = device.adapter_info();
        let bind_group_layout = GLOBAL_CACHE.get_or_create_layout(
            device.device(),
            adapter_info,
            layout_sig,
            Some("Add Layout"),
        );

        // Create bind group (must be per-call - references specific buffers)
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Add Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.lhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.rhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Get cached pipeline (compiles shader on first call, reuses after)
        let pipeline = GLOBAL_CACHE.get_or_create_pipeline(
            device.device(),
            adapter_info,
            shader_source,
            layout_sig,
            "main",
            Some("Add Pipeline"),
        );

        // Encode and execute (or queue if batching)
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Add Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Add Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Use vendor-optimized workgroup size
            let workgroups = (size as u32).div_ceil(workgroup_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor with pooled buffer (auto-returns to pool on drop!)
        Ok(Tensor::from_pooled_buffer(
            output_buffer,
            self.lhs.shape().to_vec(),
            device.clone(),
        ))
    }
}

// Convenience methods on Tensor
impl Tensor {
    /// Element-wise addition
    pub fn add(&self, other: &Tensor) -> Result<Self> {
        Add::new(self.clone(), other.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_add_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let lhs = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(vec![10.0, 20.0, 30.0, 40.0, 50.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        let expected = vec![11.0, 22.0, 33.0, 44.0, 55.0];
        for (i, (&r, &e)) in result.iter().zip(expected.iter()).enumerate() {
            assert!((r - e).abs() < 1e-6, "Mismatch at {}: {} vs {}", i, r, e);
        }
    }

    #[tokio::test]
    async fn test_add_edge_cases() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Very small values, zero, negatives
        let lhs = Tensor::from_vec_on(vec![-1e-6, 0.0, 1e-6, -1.0, 1.0], vec![5], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(vec![1e-6, 0.0, -1e-6, 1.0, -1.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        assert!((result[0] - 0.0).abs() < 1e-12); // -1e-6 + 1e-6 = 0
        assert_eq!(result[1], 0.0); // 0 + 0 = 0
        assert!((result[2] - 0.0).abs() < 1e-12); // 1e-6 + (-1e-6) = 0
        assert_eq!(result[3], 0.0); // -1 + 1 = 0
        assert_eq!(result[4], 0.0); // 1 + (-1) = 0
    }

    #[tokio::test]
    async fn test_add_boundary() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Infinities and large values
        let lhs = Tensor::from_vec_on(
            vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
            vec![5],
            device.clone(),
        )
        .await
        .unwrap();

        let rhs = Tensor::from_vec_on(vec![100.0, 1e10, 0.0, -1e10, 100.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        assert!(result[0].is_infinite() && result[0].is_sign_negative()); // -inf + 100 = -inf
        assert_eq!(result[1], 0.0); // -1e10 + 1e10 = 0 (approximately)
        assert_eq!(result[2], 0.0); // 0 + 0 = 0
        assert_eq!(result[3], 0.0); // 1e10 + (-1e10) = 0 (approximately)
        assert!(result[4].is_infinite() && result[4].is_sign_positive()); // inf + 100 = inf
    }

    #[tokio::test]
    async fn test_add_large_tensor() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let size = 1000;
        let lhs_data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let rhs_data: Vec<f32> = (0..size).map(|i| (size - i) as f32).collect();

        let lhs = Tensor::from_vec_on(lhs_data.clone(), vec![size], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(rhs_data.clone(), vec![size], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        // All should equal size
        for (i, &val) in result.iter().enumerate() {
            assert!(
                (val - size as f32).abs() < 1e-4,
                "Mismatch at {}: {}",
                i,
                val
            );
        }
    }

    #[tokio::test]
    async fn test_add_precision() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let lhs_data = vec![-5.0, -2.5, -1.0, 0.0, 1.0, 2.5, 5.0];
        let rhs_data = vec![2.0, 1.5, 0.5, 0.0, -0.5, -1.5, -2.0];

        let lhs = Tensor::from_vec_on(lhs_data.clone(), vec![7], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(rhs_data.clone(), vec![7], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let gpu_result = output.to_vec().unwrap();

        // CPU reference
        let cpu_result: Vec<f32> = lhs_data
            .iter()
            .zip(rhs_data.iter())
            .map(|(&a, &b)| a + b)
            .collect();

        // Should be exact for addition
        for (i, (&gpu, &cpu)) in gpu_result.iter().zip(cpu_result.iter()).enumerate() {
            assert!(
                (gpu - cpu).abs() < 1e-6,
                "Error at {}: GPU={}, CPU={}",
                i,
                gpu,
                cpu
            );
        }
    }
}
