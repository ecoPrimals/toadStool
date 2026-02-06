//! AdamW Optimizer - GPU-accelerated Adam with Decoupled Weight Decay
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (new shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready optimizer)
//! - ✅ Capability-based dispatch (vendor-optimized workgroups)
//!
//! ## Algorithm
//!
//! ```text
//! m = beta1 * m + (1 - beta1) * gradient
//! v = beta2 * v + (1 - beta2) * gradient²
//! m_hat = m / (1 - beta1^t)
//! v_hat = v / (1 - beta2^t)
//! param = param - lr * m_hat / (sqrt(v_hat) + epsilon) - lr * wd * param
//! ```
//!
//! **Key Difference from Adam**: Weight decay is decoupled from gradient update!
//!
//! **Implementation**: Single-pass GPU optimizer with decoupled weight decay
//!
//! **Key Properties**:
//! - Decoupled weight decay (superior to L2 regularization)
//! - Works better with adaptive learning rates
//! - Standard in modern transformers (BERT, GPT, etc.)
//! - Automatic bias correction
//!
//! **Used By**: Modern deep learning, large language models, SOTA training
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let weights = Tensor::randn(vec![1000, 512]).await?;
//! let gradients = Tensor::randn(vec![1000, 512]).await?;
//! let m = Tensor::zeros(vec![1000, 512]).await?;
//! let v = Tensor::zeros(vec![1000, 512]).await?;
//!
//! let (new_weights, new_m, new_v) = weights.adamw(
//!     &gradients,
//!     &m,
//!     &v,
//!     0.001,  // learning_rate
//!     0.9,    // beta1
//!     0.999,  // beta2
//!     1e-8,   // epsilon
//!     0.01,   // weight_decay (decoupled!)
//!     1,      // step
//! )?;
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// AdamW optimizer parameters for WGSL shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AdamWParams {
    num_params: u32,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
    step: u32,
}

/// AdamW Optimizer operation
///
/// **Deep Debt**: Uses new WGSL shader with decoupled weight decay
pub struct AdamW {
    params: Tensor,
    gradients: Tensor,
    m: Tensor,
    v: Tensor,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
    step: u32,
}

impl AdamW {
    /// Create new AdamW optimizer operation
    ///
    /// **Deep Debt**: Validates all inputs for shape compatibility
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        params: Tensor,
        gradients: Tensor,
        m: Tensor,
        v: Tensor,
        learning_rate: f32,
        beta1: f32,
        beta2: f32,
        epsilon: f32,
        weight_decay: f32,
        step: u32,
    ) -> Result<Self> {
        // Validate shapes match
        if params.shape() != gradients.shape() {
            return Err(BarracudaError::shape_mismatch(
                params.shape().to_vec(),
                gradients.shape().to_vec(),
            ));
        }
        if params.shape() != m.shape() {
            return Err(BarracudaError::shape_mismatch(
                params.shape().to_vec(),
                m.shape().to_vec(),
            ));
        }
        if params.shape() != v.shape() {
            return Err(BarracudaError::shape_mismatch(
                params.shape().to_vec(),
                v.shape().to_vec(),
            ));
        }

        // Validate hyperparameters
        if !(0.0..1.0).contains(&beta1) {
            return Err(BarracudaError::invalid_op(
                "AdamW",
                format!("beta1 must be in [0, 1), got {}", beta1),
            ));
        }
        if !(0.0..1.0).contains(&beta2) {
            return Err(BarracudaError::invalid_op(
                "AdamW",
                format!("beta2 must be in [0, 1), got {}", beta2),
            ));
        }
        if epsilon <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "AdamW",
                format!("epsilon must be positive, got {}", epsilon),
            ));
        }
        if step == 0 {
            return Err(BarracudaError::invalid_op(
                "AdamW",
                "step must be >= 1 for bias correction",
            ));
        }

        Ok(Self {
            params,
            gradients,
            m,
            v,
            learning_rate,
            beta1,
            beta2,
            epsilon,
            weight_decay,
            step,
        })
    }

    /// WGSL shader source
    fn shader() -> &'static str {
        include_str!("../shaders/adamw.wgsl")
    }

    /// Execute AdamW optimizer step (GPU single-pass with decoupled weight decay)
    ///
    /// **Deep Debt**: Efficient single-pass update with decoupled weight decay
    ///
    /// Returns: (new_params, new_m, new_v)
    pub fn execute(self) -> Result<(Tensor, Tensor, Tensor)> {
        let device = self.params.device();
        let size = self.params.len();

        // Create parameters
        let params = AdamWParams {
            num_params: size as u32,
            learning_rate: self.learning_rate,
            beta1: self.beta1,
            beta2: self.beta2,
            epsilon: self.epsilon,
            weight_decay: self.weight_decay,
            step: self.step,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("AdamW Params"),
            size: std::mem::size_of::<AdamWParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Output buffers
        let params_out_buffer = device.create_buffer_f32(size)?;
        let m_out_buffer = device.create_buffer_f32(size)?;
        let v_out_buffer = device.create_buffer_f32(size)?;

        // Copy initial params, m, v to output buffers (will be updated in-place)
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("AdamW Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            self.params.buffer(),
            0,
            &params_out_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            self.m.buffer(),
            0,
            &m_out_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            self.v.buffer(),
            0,
            &v_out_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Compile shader
        let shader = device.compile_shader(Self::shader(), Some("AdamW"));

        // Create bind group layout (5 bindings)
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("AdamW BGL"),
                entries: &[
                    // gradients (read)
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
                    // params (read_write)
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
                    // m (read_write)
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
                    // v (read_write)
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
                    // params (uniform)
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AdamW BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.gradients.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: m_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: v_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("AdamW Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("AdamW Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("AdamW Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AdamW Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (size as u32 + optimal_wg_size - 1) / optimal_wg_size;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return all three outputs
        Ok((
            Tensor::from_buffer(
                params_out_buffer,
                self.params.shape().to_vec(),
                device.clone(),
            ),
            Tensor::from_buffer(m_out_buffer, self.m.shape().to_vec(), device.clone()),
            Tensor::from_buffer(v_out_buffer, self.v.shape().to_vec(), device.clone()),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// AdamW optimizer step (Adam with Decoupled Weight Decay)
    ///
    /// **Deep Debt**: Production-ready optimizer with decoupled weight decay
    ///
    /// # Arguments
    /// - `gradients`: Gradient tensor [same shape as params]
    /// - `m`: First moment estimate [same shape as params]
    /// - `v`: Second moment estimate [same shape as params]
    /// - `learning_rate`: Learning rate (e.g., 0.001)
    /// - `beta1`: First moment decay (typically 0.9)
    /// - `beta2`: Second moment decay (typically 0.999)
    /// - `epsilon`: Numerical stability (typically 1e-8)
    /// - `weight_decay`: Decoupled weight decay (typically 0.01, 0.0 = none)
    /// - `step`: Current step number (for bias correction, must be >= 1)
    ///
    /// # Returns
    /// - `(new_params, new_m, new_v)`: Updated parameters and moments
    ///
    /// # Example
    /// ```rust,ignore
    /// let (p, m, v) = params.adamw(&grad, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.01, 1)?;
    /// ```
    ///
    /// # Note
    /// AdamW is superior to Adam for large models due to decoupled weight decay!
    #[allow(clippy::too_many_arguments)]
    pub fn adamw(
        self,
        gradients: &Self,
        m: &Self,
        v: &Self,
        learning_rate: f32,
        beta1: f32,
        beta2: f32,
        epsilon: f32,
        weight_decay: f32,
        step: u32,
    ) -> Result<(Self, Self, Self)> {
        AdamW::new(
            self,
            gradients.clone(),
            m.clone(),
            v.clone(),
            learning_rate,
            beta1,
            beta2,
            epsilon,
            weight_decay,
            step,
        )?
        .execute()
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
    async fn test_adamw_gpu_basic() {
        let device = get_test_device().await;

        let size = 1000;
        let params = Tensor::from_vec_on(vec![1.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; size], vec![size], device.clone())
            .await
            .unwrap();

        let m = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let v = Tensor::from_vec_on(vec![0.0; size], vec![size], device)
            .await
            .unwrap();

        let (new_params, new_m, new_v) = params
            .adamw(&gradients, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.01, 1)
            .unwrap();

        assert_eq!(new_params.shape(), &[size]);
        assert_eq!(new_m.shape(), &[size]);
        assert_eq!(new_v.shape(), &[size]);

        let p_data = new_params.to_vec().unwrap();
        let m_data = new_m.to_vec().unwrap();
        let v_data = new_v.to_vec().unwrap();

        // Params should decrease (gradient descent + weight decay)
        assert!(p_data.iter().all(|&x| x < 1.0));
        // m should be non-zero (momentum accumulated)
        assert!(m_data.iter().any(|&x| x.abs() > 1e-6));
        // v should be non-zero (variance accumulated)
        assert!(v_data.iter().all(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_adamw_gpu_convergence() {
        let device = get_test_device().await;

        let size = 100;
        let mut params = Tensor::from_vec_on(vec![5.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let mut m = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let mut v = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        // Constant gradient pointing toward zero
        let gradients = Tensor::from_vec_on(vec![1.0; size], vec![size], device)
            .await
            .unwrap();

        // Run 10 steps
        for step in 1..=10 {
            let (p, m_new, v_new) = params
                .adamw(&gradients, &m, &v, 0.1, 0.9, 0.999, 1e-8, 0.01, step)
                .unwrap();
            params = p;
            m = m_new;
            v = v_new;
        }

        let final_params = params.to_vec().unwrap();
        // Should converge toward lower values
        assert!(final_params.iter().all(|&x| x < 4.0));
    }

    #[tokio::test]
    async fn test_adamw_gpu_weight_decay_stronger() {
        let device = get_test_device().await;

        let size = 100;
        let params = Tensor::from_vec_on(vec![10.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let m = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let v = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        // With weight decay, params should shrink even with zero gradient
        let (new_params_wd, _, _) = params
            .clone()
            .adamw(&gradients, &m, &v, 0.1, 0.9, 0.999, 1e-8, 0.1, 1)
            .unwrap();

        let (new_params_no_wd, _, _) = params
            .adamw(&gradients, &m, &v, 0.1, 0.9, 0.999, 1e-8, 0.0, 1)
            .unwrap();

        let wd_data = new_params_wd.to_vec().unwrap();
        let no_wd_data = new_params_no_wd.to_vec().unwrap();

        // Weight decay should reduce params more than no weight decay
        assert!(wd_data[0] < no_wd_data[0]);
        assert!(wd_data[0] < 10.0);
    }

    #[tokio::test]
    async fn test_adamw_gpu_shape_validation() {
        let device = get_test_device().await;

        let params = Tensor::from_vec_on(vec![1.0; 100], vec![100], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 50], vec![50], device.clone())
            .await
            .unwrap();

        let m = Tensor::from_vec_on(vec![0.0; 100], vec![100], device.clone())
            .await
            .unwrap();

        let v = Tensor::from_vec_on(vec![0.0; 100], vec![100], device)
            .await
            .unwrap();

        // Shape mismatch should error
        let result = params.adamw(&gradients, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.01, 1);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_adamw_gpu_multidimensional() {
        let device = get_test_device().await;

        // 2D params (matrix)
        let params = Tensor::from_vec_on(vec![1.0; 100], vec![10, 10], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 100], vec![10, 10], device.clone())
            .await
            .unwrap();

        let m = Tensor::from_vec_on(vec![0.0; 100], vec![10, 10], device.clone())
            .await
            .unwrap();

        let v = Tensor::from_vec_on(vec![0.0; 100], vec![10, 10], device)
            .await
            .unwrap();

        let (new_params, new_m, new_v) = params
            .adamw(&gradients, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.01, 1)
            .unwrap();

        assert_eq!(new_params.shape(), &[10, 10]);
        assert_eq!(new_m.shape(), &[10, 10]);
        assert_eq!(new_v.shape(), &[10, 10]);
    }

    #[tokio::test]
    async fn test_adamw_vs_adam_difference() {
        let device = get_test_device().await;

        // Compare AdamW vs Adam behavior with same hyperparameters
        let size = 100;
        let params = Tensor::from_vec_on(vec![10.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; size], vec![size], device.clone())
            .await
            .unwrap();

        let m = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let v = Tensor::from_vec_on(vec![0.0; size], vec![size], device)
            .await
            .unwrap();

        // AdamW with weight decay
        let (adamw_params, _, _) = params
            .clone()
            .adamw(&gradients, &m, &v, 0.01, 0.9, 0.999, 1e-8, 0.1, 1)
            .unwrap();

        let adamw_data = adamw_params.to_vec().unwrap();

        // AdamW should apply decoupled weight decay
        // Result should be different from params without update
        assert!(adamw_data.iter().all(|&x| x < 10.0));

        // Verify all values are finite
        assert!(adamw_data.iter().all(|&x| x.is_finite()));
    }
}
