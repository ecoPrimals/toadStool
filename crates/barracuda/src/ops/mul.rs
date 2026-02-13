//! Element-wise multiplication
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (universal compute)
//! - ✅ Capability-based dispatch (vendor-optimized)
//! - ✅ Vendor-specific workgroup sizes (NVIDIA: 64, AMD: 128)
//! - ✅ Pipeline caching (compile once, dispatch many)
//! - ✅ Buffer pooling (zero allocation after warmup)
//!
//! Formula: C = A * B (element-wise, Hadamard product)

use crate::device::pipeline_cache::{BindGroupLayoutSignature, GLOBAL_CACHE};
use crate::device::tensor_context::get_device_context;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Shader source optimized for NVIDIA GPUs (WG=64)
const SHADER_WG64: &str = include_str!("../shaders/math/elementwise_mul_wg64.wgsl");

/// Shader source optimized for AMD GPUs (WG=128)  
const SHADER_WG128: &str = include_str!("../shaders/math/elementwise_mul_wg128.wgsl");

/// Default shader (WG=256, fallback)
const SHADER_DEFAULT: &str = include_str!("../shaders/math/elementwise_mul.wgsl");

/// Element-wise multiplication operation
pub struct Mul {
    lhs: Tensor,
    rhs: Tensor,
}

impl Mul {
    /// Create Mul operation
    pub fn new(lhs: Tensor, rhs: Tensor) -> Result<Self> {
        if lhs.shape() != rhs.shape() {
            return Err(BarracudaError::shape_mismatch(
                lhs.shape().to_vec(),
                rhs.shape().to_vec(),
            ));
        }
        Ok(Self { lhs, rhs })
    }

    /// Select vendor-optimized shader based on GPU and tensor size
    fn wgsl_shader(device_name: &str, size: usize) -> (&'static str, u32) {
        let lower = device_name.to_lowercase();
        let max_dispatch = 65535u32;
        let (nvidia_wg, amd_wg) = (64u32, 128u32);
        
        if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("rtx") || lower.contains("gtx") {
            let needed = (size as u32).div_ceil(nvidia_wg);
            if needed <= max_dispatch { (SHADER_WG64, nvidia_wg) } else { (SHADER_DEFAULT, 256) }
        } else if lower.contains("amd") || lower.contains("radeon") || lower.contains("radv") {
            let needed = (size as u32).div_ceil(amd_wg);
            if needed <= max_dispatch { (SHADER_WG128, amd_wg) } else { (SHADER_DEFAULT, 256) }
        } else {
            (SHADER_DEFAULT, 256)
        }
    }

    /// Execute multiplication on tensors
    ///
    /// Uses cached shader and pipeline for fast repeated calls.
    /// Output buffer is acquired from pool for zero-allocation steady-state.
    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size = self.lhs.len();

        // Get device context for buffer pooling
        let ctx = get_device_context(device);

        // Select vendor-optimized shader based on GPU and tensor size
        let device_name = device.name();
        let (shader_source, workgroup_size) = Self::wgsl_shader(device_name, size);

        // Acquire output buffer from pool (key optimization!)
        let output_buffer = ctx.acquire_output_buffer(size);

        // Get cached bind group layout
        let layout_sig = BindGroupLayoutSignature::elementwise_binary();
        let adapter_info = device.adapter_info();
        let bind_group_layout = GLOBAL_CACHE.get_or_create_layout(
            device.device(),
            adapter_info,
            layout_sig,
            Some("Mul Layout"),
        );

        // Create bind group (must be per-call - references specific buffers)
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mul Bind Group"),
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
            Some("Mul Pipeline"),
        );

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Mul Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Mul Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Use vendor-optimized workgroup size
            let workgroups = (size as u32).div_ceil(workgroup_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.lhs.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Element-wise multiplication
    pub fn mul(&self, other: &Tensor) -> Result<Self> {
        Mul::new(self.clone(), other.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_mul_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let lhs = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(vec![2.0, 3.0, 4.0, 5.0, 6.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.mul(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        let expected = vec![2.0, 6.0, 12.0, 20.0, 30.0];
        for (&r, &e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_mul_edge_cases() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Zero multiplication, small values
        let lhs = Tensor::from_vec_on(vec![0.0, 1e-6, -1e-6, 1.0, -1.0], vec![5], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(vec![5.0, 1e6, 1e6, 0.0, -0.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.mul(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        assert_eq!(result[0], 0.0); // 0 * 5 = 0
        assert!((result[1] - 1.0).abs() < 1e-4); // 1e-6 * 1e6 = 1
        assert!((result[2] + 1.0).abs() < 1e-4); // -1e-6 * 1e6 = -1
        assert_eq!(result[3], 0.0); // 1 * 0 = 0
    }

    #[tokio::test]
    async fn test_mul_boundary() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let lhs = Tensor::from_vec_on(
            vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
            vec![5],
            device.clone(),
        )
        .await
        .unwrap();

        let rhs = Tensor::from_vec_on(vec![2.0, 2.0, 2.0, 2.0, 2.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.mul(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        assert!(result[0].is_infinite() && result[0].is_sign_negative());
        assert_eq!(result[2], 0.0);
        assert!(result[4].is_infinite() && result[4].is_sign_positive());
    }

    #[tokio::test]
    async fn test_mul_large_tensor() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let size = 1000;
        let lhs_data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let rhs_data = vec![2.0; size];

        let lhs = Tensor::from_vec_on(lhs_data.clone(), vec![size], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(rhs_data, vec![size], device)
            .await
            .unwrap();

        let output = lhs.mul(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        for (i, &val) in result.iter().enumerate() {
            assert!((val - (i as f32) * 2.0).abs() < 1e-4);
        }
    }

    #[tokio::test]
    async fn test_mul_precision() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let lhs_data = vec![-5.0, -2.5, -1.0, 0.0, 1.0, 2.5, 5.0];
        let rhs_data = vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        let lhs = Tensor::from_vec_on(lhs_data.clone(), vec![7], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(rhs_data.clone(), vec![7], device)
            .await
            .unwrap();

        let output = lhs.mul(&rhs).unwrap();
        let gpu_result = output.to_vec().unwrap();

        let cpu_result: Vec<f32> = lhs_data
            .iter()
            .zip(rhs_data.iter())
            .map(|(&a, &b)| a * b)
            .collect();

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
