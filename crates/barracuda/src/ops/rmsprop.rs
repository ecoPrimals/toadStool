//! RMSprop Optimizer - GPU-accelerated Root Mean Square Propagation
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (existing shader evolved)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//! - ✅ Modern idiomatic Rust (no traits, direct impl)
//! - ✅ Capability-based dispatch (vendor-optimized workgroups)
//!
//! ## Algorithm
//!
//! ```text
//! E[g²] = α * E[g²] + (1 - α) * g²
//! w = w - lr * g / (sqrt(E[g²]) + ε)
//! ```
//!
//! **Key Properties**:
//! - Adaptive learning rate per parameter
//! - Uses moving average of squared gradients
//! - More stable than AdaGrad (doesn't monotonically decrease)
//! - Popular for RNNs and non-stationary problems
//!
//! **Parameters**:
//! - `learning_rate`: Step size, typically 0.001-0.01
//! - `alpha` (α): Decay rate for moving average, typically 0.99
//! - `epsilon` (ε): Numerical stability constant, typically 1e-8
//!
//! **Used By**: RNNs, non-stationary optimization problems
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let weights = Tensor::randn(vec![1000]).await?;
//! let gradients = Tensor::randn(vec![1000]).await?;
//!
//! // First step
//! let (w1, sq_avg1) = weights.rmsprop_step(&gradients, 0.001, 0.99, None)?;
//!
//! // Subsequent steps
//! let (w2, sq_avg2) = w1.rmsprop_step(&gradients, 0.001, 0.99, Some(&sq_avg1))?;
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RMSpropParams {
    learning_rate: f32,
    alpha: f32,
    epsilon: f32,
    weight_decay: f32,
}

pub struct RMSprop {
    weights: Tensor,
    gradients: Tensor,
    sq_avg: Option<Tensor>,
    learning_rate: f32,
    alpha: f32,
}

impl RMSprop {
    pub fn new(
        weights: Tensor,
        gradients: Tensor,
        learning_rate: f32,
        alpha: f32,
        sq_avg: Option<Tensor>,
    ) -> Result<Self> {
        // Validate shapes match
        if weights.shape() != gradients.shape() {
            return Err(BarracudaError::shape_mismatch(
                weights.shape().to_vec(),
                gradients.shape().to_vec(),
            ));
        }

        // Validate learning rate is positive
        if learning_rate <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "rmsprop",
                "learning_rate must be positive",
            ));
        }

        // Validate alpha in valid range
        if !(0.0..=1.0).contains(&alpha) {
            return Err(BarracudaError::invalid_op(
                "rmsprop",
                "alpha must be in range [0.0, 1.0]",
            ));
        }

        // Validate sq_avg shape if provided
        if let Some(ref sq) = sq_avg {
            if sq.shape() != weights.shape() {
                return Err(BarracudaError::shape_mismatch(
                    sq.shape().to_vec(),
                    weights.shape().to_vec(),
                ));
            }
        }

        Ok(Self {
            weights,
            gradients,
            sq_avg,
            learning_rate,
            alpha,
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/rmsprop.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor)> {
        let device = self.weights.device();
        let size = self.weights.shape().iter().product::<usize>();

        let params = RMSpropParams {
            learning_rate: self.learning_rate,
            alpha: self.alpha,
            epsilon: 1e-8,
            weight_decay: 0.0,
        };

        // Create sq_avg buffer if not provided
        let sq_avg_in = if let Some(ref sq) = self.sq_avg {
            sq.buffer()
        } else {
            let zeros = vec![0.0f32; size];
            &device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("rmsprop_sq_avg_zeros"),
                    contents: bytemuck::cast_slice(&zeros),
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };

        let weights_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rmsprop_weights_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let sq_avg_out_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rmsprop_sq_avg_out"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("rmsprop_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("rmsprop_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("rmsprop_bind_group_layout"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
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
                    label: Some("rmsprop_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("rmsprop_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rmsprop_bind_group"),
            layout: &bind_group_layout,
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
                    resource: sq_avg_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: weights_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: sq_avg_out_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rmsprop_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rmsprop_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = ((size as u32) + optimal_wg_size - 1) / optimal_wg_size;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        let updated_weights = Tensor::from_buffer(
            weights_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );

        let updated_sq_avg = Tensor::from_buffer(
            sq_avg_out_buffer,
            self.weights.shape().to_vec(),
            device.clone(),
        );

        Ok((updated_weights, updated_sq_avg))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION (MODERN IDIOMATIC RUST)
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// RMSprop optimizer step - adaptive learning rate optimizer
    ///
    /// **Deep Debt**: Essential for RNNs and non-stationary problems
    ///
    /// # Arguments
    /// - `gradients`: Gradient tensor [same shape as weights]
    /// - `learning_rate`: Step size, typically 0.001-0.01
    /// - `alpha`: Decay rate for moving average, typically 0.99
    /// - `sq_avg`: Accumulated squared gradients (None for first step)
    ///
    /// # Returns
    /// - Tuple: (updated_weights, updated_sq_avg)
    ///
    /// # Example
    /// ```rust,ignore
    /// // First step
    /// let (w1, sq1) = weights.rmsprop_step(&grads, 0.001, 0.99, None)?;
    ///
    /// // Subsequent steps
    /// let (w2, sq2) = w1.rmsprop_step(&grads, 0.001, 0.99, Some(&sq1))?;
    /// ```
    ///
    /// # Note
    /// - Adaptive learning rate per parameter
    /// - Popular for RNNs
    /// - learning_rate must be positive
    /// - alpha must be in [0.0, 1.0]
    pub fn rmsprop_step(
        self,
        gradients: &Self,
        learning_rate: f32,
        alpha: f32,
        sq_avg: Option<&Self>,
    ) -> Result<(Self, Self)> {
        RMSprop::new(
            self,
            gradients.clone(),
            learning_rate,
            alpha,
            sq_avg.cloned(),
        )?
        .execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_rmsprop_basic() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1, 0.2, 0.3, 0.4], vec![4], device.clone())
            .await
            .unwrap();

        let (updated_weights, updated_sq_avg) =
            weights.rmsprop_step(&gradients, 0.001, 0.99, None).unwrap();

        let result = updated_weights.to_vec().unwrap();
        let sq_avg = updated_sq_avg.to_vec().unwrap();

        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(sq_avg.iter().all(|&x| x.is_finite()));
        assert!(result[0] < 1.0, "Expected descent, got {}", result[0]);
    }

    #[tokio::test]
    async fn test_rmsprop_accumulation() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 4], vec![4], device.clone())
            .await
            .unwrap();

        // First step
        let (weights1, sq_avg1) = weights.rmsprop_step(&gradients, 0.001, 0.99, None).unwrap();

        let sq1 = sq_avg1.to_vec().unwrap();
        assert!(sq1.iter().all(|&x| x >= 0.0));

        // Second step with accumulated state
        let (weights2, sq_avg2) = weights1
            .rmsprop_step(&gradients, 0.001, 0.99, Some(&sq_avg1))
            .unwrap();

        let result = weights2.to_vec().unwrap();
        let sq2 = sq_avg2.to_vec().unwrap();

        assert!(result.iter().all(|&x| x.is_finite()));
        assert!(sq2.iter().all(|&x| x >= sq1[0])); // Should accumulate
    }

    #[tokio::test]
    async fn test_rmsprop_different_alpha() {
        let device = get_test_device().await;

        let weights1 = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let weights2 = Tensor::from_vec_on(vec![1.0; 4], vec![4], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.1; 4], vec![4], device.clone())
            .await
            .unwrap();

        // Low alpha (less history)
        let (updated1, _) = weights1.rmsprop_step(&gradients, 0.001, 0.5, None).unwrap();

        // High alpha (more history)
        let (updated2, _) = weights2
            .rmsprop_step(&gradients, 0.001, 0.99, None)
            .unwrap();

        let result1 = updated1.to_vec().unwrap();
        let result2 = updated2.to_vec().unwrap();

        assert!(result1.iter().all(|&x| x.is_finite()));
        assert!(result2.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_rmsprop_validation() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![1.0; 10], vec![10], device.clone())
            .await
            .unwrap();
        let gradients = Tensor::from_vec_on(vec![0.1; 5], vec![5], device.clone())
            .await
            .unwrap();
        let grads_correct = Tensor::from_vec_on(vec![0.1; 10], vec![10], device.clone())
            .await
            .unwrap();

        // Shape mismatch
        assert!(weights
            .clone()
            .rmsprop_step(&gradients, 0.001, 0.99, None)
            .is_err());

        // Invalid learning rate
        assert!(weights
            .clone()
            .rmsprop_step(&grads_correct, -0.001, 0.99, None)
            .is_err());
        assert!(weights
            .clone()
            .rmsprop_step(&grads_correct, 0.0, 0.99, None)
            .is_err());

        // Invalid alpha
        assert!(weights
            .clone()
            .rmsprop_step(&grads_correct, 0.001, -0.1, None)
            .is_err());
        assert!(weights
            .clone()
            .rmsprop_step(&grads_correct, 0.001, 1.5, None)
            .is_err());
    }

    #[tokio::test]
    async fn test_rmsprop_large_batch() {
        let device = get_test_device().await;

        let size = 128;
        let weights = Tensor::from_vec_on(vec![1.0; size], vec![size], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![0.01; size], vec![size], device.clone())
            .await
            .unwrap();

        let (updated_weights, updated_sq_avg) =
            weights.rmsprop_step(&gradients, 0.001, 0.99, None).unwrap();

        let result = updated_weights.to_vec().unwrap();
        let sq_avg = updated_sq_avg.to_vec().unwrap();

        assert_eq!(result.len(), size);
        assert_eq!(sq_avg.len(), size);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_rmsprop_multi_step() {
        let device = get_test_device().await;

        let weights = Tensor::from_vec_on(vec![10.0, 20.0], vec![2], device.clone())
            .await
            .unwrap();

        let gradients = Tensor::from_vec_on(vec![1.0, 2.0], vec![2], device.clone())
            .await
            .unwrap();

        // Step 1
        let (weights1, sq1) = weights.rmsprop_step(&gradients, 0.01, 0.99, None).unwrap();
        let result1 = weights1.to_vec().unwrap();

        assert!(result1[0] < 10.0, "Expected descent, got {}", result1[0]);
        assert!(result1[1] < 20.0, "Expected descent, got {}", result1[1]);

        // Step 2 with accumulated state
        let (weights2, _sq2) = weights1
            .rmsprop_step(&gradients, 0.01, 0.99, Some(&sq1))
            .unwrap();
        let result2 = weights2.to_vec().unwrap();

        // Should continue descending
        assert!(result2[0] < result1[0]);
        assert!(result2[1] < result1[1]);
    }
}
