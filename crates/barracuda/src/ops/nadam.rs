//! NAdam Optimizer - GPU-accelerated Nesterov-accelerated Adam
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (uses existing shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready optimizer)
//!
//! ## Algorithm
//!
//! ```text
//! m = beta1 * m + (1 - beta1) * gradient
//! v = beta2 * v + (1 - beta2) * gradient²
//! m_hat = m / (1 - beta1^t)
//! v_hat = v / (1 - beta2^t)
//! gradient_nesterov = (beta1 * m_hat + (1 - beta1) * gradient) / (1 - beta1^t)
//! weight = weight - learning_rate * gradient_nesterov / (sqrt(v_hat) + epsilon)
//! ```
//!
//! **Implementation**: Single-pass GPU optimizer with Nesterov momentum
//!
//! **Key Properties**:
//! - Combines Adam with Nesterov momentum
//! - Faster convergence than standard Adam
//! - Automatic bias correction
//! - Optional weight decay (L2 regularization)
//!
//! **Used By**: Modern deep learning training, faster than Adam
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
//! let (new_weights, new_m, new_v) = weights.nadam(
//!     &gradients,
//!     &m,
//!     &v,
//!     0.001,  // learning_rate
//!     0.9,    // beta1
//!     0.999,  // beta2
//!     1e-8,   // epsilon
//!     0.0,    // weight_decay
//!     1,      // step
//! )?;
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// NAdam optimizer parameters for WGSL shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct NadamParams {
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
    step: u32,
    _padding: [u32; 2], // Explicit padding for 16-byte alignment
}

/// NAdam Optimizer operation
///
/// **Deep Debt**: Uses existing WGSL shader with Nesterov momentum
pub struct Nadam {
    weights: Tensor,
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

impl Nadam {
    /// Create new NAdam optimizer operation
    ///
    /// **Deep Debt**: Validates all inputs for shape compatibility
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        weights: Tensor,
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
        if weights.shape() != gradients.shape() {
            return Err(BarracudaError::shape_mismatch(
                weights.shape().to_vec(),
                gradients.shape().to_vec(),
            ));
        }
        if weights.shape() != m.shape() {
            return Err(BarracudaError::shape_mismatch(
                weights.shape().to_vec(),
                m.shape().to_vec(),
            ));
        }
        if weights.shape() != v.shape() {
            return Err(BarracudaError::shape_mismatch(
                weights.shape().to_vec(),
                v.shape().to_vec(),
            ));
        }

        // Validate hyperparameters
        if !(0.0..1.0).contains(&beta1) {
            return Err(BarracudaError::invalid_op(
                "NAdam",
                format!("beta1 must be in [0, 1), got {}", beta1),
            ));
        }
        if !(0.0..1.0).contains(&beta2) {
            return Err(BarracudaError::invalid_op(
                "NAdam",
                format!("beta2 must be in [0, 1), got {}", beta2),
            ));
        }
        if epsilon <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "NAdam",
                format!("epsilon must be positive, got {}", epsilon),
            ));
        }
        if step == 0 {
            return Err(BarracudaError::invalid_op(
                "NAdam",
                "step must be >= 1 for bias correction",
            ));
        }

        Ok(Self {
            weights,
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
        include_str!("../shaders/nadam.wgsl")
    }

    /// Execute NAdam optimizer step (GPU single-pass)
    ///
    /// **Deep Debt**: Efficient single-pass update with Nesterov momentum
    ///
    /// Returns: (new_weights, new_m, new_v)
    pub fn execute(self) -> Result<(Tensor, Tensor, Tensor)> {
        let device = self.weights.device();
        let size = self.weights.len();

        // Create parameters
        let params = NadamParams {
            learning_rate: self.learning_rate,
            beta1: self.beta1,
            beta2: self.beta2,
            epsilon: self.epsilon,
            weight_decay: self.weight_decay,
            step: self.step,
            _padding: [0, 0],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("NAdam Params"),
            size: std::mem::size_of::<NadamParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Output buffers
        let weights_out_buffer = device.create_buffer_f32(size)?;
        let m_out_buffer = device.create_buffer_f32(size)?;
        let v_out_buffer = device.create_buffer_f32(size)?;

        // Compile shader
        let shader = device.compile_shader(Self::shader(), Some("NAdam"));

        // Create bind group layout (7 bindings)
        let bgl = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("NAdam BGL"),
            entries: &[
                // weights (read)
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
                // gradients (read)
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
                // m_in (read)
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
                // v_in (read)
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
                // weights_out (write)
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
                // m_out (write)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // v_out (write)
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
                // params (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
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
            label: Some("NAdam BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.weights.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.gradients.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.m.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.v.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: m_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: v_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NAdam Pipeline Layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NAdam Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("NAdam Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NAdam Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            
            // Workgroups: size=256 per shader
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return all three outputs
        Ok((
            Tensor::from_buffer(weights_out_buffer, self.weights.shape().to_vec(), device.clone()),
            Tensor::from_buffer(m_out_buffer, self.m.shape().to_vec(), device.clone()),
            Tensor::from_buffer(v_out_buffer, self.v.shape().to_vec(), device.clone()),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// NAdam optimizer step (Nesterov-accelerated Adam)
    ///
    /// **Deep Debt**: Production-ready optimizer with Nesterov momentum
    ///
    /// # Arguments
    /// - `gradients`: Gradient tensor [same shape as weights]
    /// - `m`: First moment estimate [same shape as weights]
    /// - `v`: Second moment estimate [same shape as weights]
    /// - `learning_rate`: Learning rate (e.g., 0.001)
    /// - `beta1`: First moment decay (typically 0.9)
    /// - `beta2`: Second moment decay (typically 0.999)
    /// - `epsilon`: Numerical stability (typically 1e-8)
    /// - `weight_decay`: L2 regularization (0.0 = none)
    /// - `step`: Current step number (for bias correction, must be >= 1)
    ///
    /// # Returns
    /// - `(new_weights, new_m, new_v)`: Updated parameters and moments
    ///
    /// # Example
    /// ```rust,ignore
    /// let (w, m, v) = weights.nadam(&grad, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.0, 1)?;
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn nadam(
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
        Nadam::new(
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
    async fn test_nadam_gpu_basic() {
        let device = get_test_device().await;

        let size = 1000;
        let weights = Tensor::from_vec_on(vec![1.0; size], vec![size], device.clone())
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

        let (new_weights, new_m, new_v) =
            weights.nadam(&gradients, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.0, 1).unwrap();

        assert_eq!(new_weights.shape(), &[size]);
        assert_eq!(new_m.shape(), &[size]);
        assert_eq!(new_v.shape(), &[size]);

        let w_data = new_weights.to_vec().unwrap();
        let m_data = new_m.to_vec().unwrap();
        let v_data = new_v.to_vec().unwrap();

        // Weights should decrease (gradient descent)
        assert!(w_data.iter().all(|&x| x < 1.0));
        // m should be non-zero (momentum accumulated)
        assert!(m_data.iter().any(|&x| x.abs() > 1e-6));
        // v should be non-zero (variance accumulated)
        assert!(v_data.iter().all(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_nadam_gpu_convergence() {
        let device = get_test_device().await;

        let size = 100;
        let mut weights = Tensor::from_vec_on(vec![5.0; size], vec![size], device.clone())
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
            let (w, m_new, v_new) =
                weights.nadam(&gradients, &m, &v, 0.1, 0.9, 0.999, 1e-8, 0.0, step).unwrap();
            weights = w;
            m = m_new;
            v = v_new;
        }

        let final_weights = weights.to_vec().unwrap();
        // Should converge toward lower values
        assert!(final_weights.iter().all(|&x| x < 4.0));
    }

    #[tokio::test]
    async fn test_nadam_gpu_weight_decay() {
        let device = get_test_device().await;

        let size = 100;
        let weights = Tensor::from_vec_on(vec![10.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let m = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let v = Tensor::from_vec_on(vec![0.0; size], vec![size], device)
            .await
            .unwrap();

        // With weight decay, weights should shrink even with zero gradient
        let (new_weights, _, _) =
            weights.nadam(&gradients, &m, &v, 0.1, 0.9, 0.999, 1e-8, 0.01, 1).unwrap();

        let w_data = new_weights.to_vec().unwrap();
        // Weight decay should reduce weights
        assert!(w_data.iter().all(|&x| x < 10.0));
    }

    #[tokio::test]
    async fn test_nadam_gpu_shape_validation() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0; 100], vec![100], device.clone())
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
        let result = weights.nadam(&gradients, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.0, 1);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_nadam_gpu_multidimensional() {
        let device = get_test_device().await;

        // 2D weights (matrix)
        let weights = Tensor::from_vec_on(vec![1.0; 100], vec![10, 10], device.clone())
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

        let (new_weights, new_m, new_v) =
            weights.nadam(&gradients, &m, &v, 0.001, 0.9, 0.999, 1e-8, 0.0, 1).unwrap();

        assert_eq!(new_weights.shape(), &[10, 10]);
        assert_eq!(new_m.shape(), &[10, 10]);
        assert_eq!(new_v.shape(), &[10, 10]);
    }
}
